// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;
use std::ops::Index;

type Reg = Rc<KHVal<LLVMHostStorage<'static>>>;

macro_rules! read_value {
    ($self:expr, $rs:expr) => {
        match *$rs.storage.borrow() {
            LLVMHostStorage::Empty => panic!("trying to use empty value"),
            LLVMHostStorage::Global(v) => {
                // check if value has previously been read
                match $self
                    .global_map
                    .borrow_mut()
                    .get_mut(&v)
                    .expect("untracked global")
                {
                    Some(cv) => *cv,
                    o @ None => {
                        let temp = $self
                            .builder
                            .build_load(v.as_pointer_value(), "")
                            .into_int_value();
                        *o = Some(temp);
                        temp
                    }
                }
            }
            LLVMHostStorage::IntV(v) => v,
            _ => panic!("not implemented"),
        }
    };
}

macro_rules! store_result {
    ($self:expr, $rd:expr, $result:expr) => {
        let mut rd_storage = $rd.storage.borrow_mut();
        match *rd_storage {
            LLVMHostStorage::Empty => *rd_storage = LLVMHostStorage::IntV($result),
            LLVMHostStorage::Global(v) => {
                // store into cache area in global_map
                *$self
                    .global_map
                    .borrow_mut()
                    .get_mut(&v)
                    .expect("untracked global") = Some($result);
            }
            _ => panic!("ssa violation: trying to write to to initialized value"),
        }
    };
}

impl CodeGen<LLVMHostStorage<'static>> for LLVMHostContext<'static> {
    fn gen_mov(&mut self, rd: Reg, rs1: Reg) {
        let result = read_value!(self, rs1);
        store_result!(self, rd, result);
    }

    fn gen_extrs(&mut self, rd: Reg, rs: Reg, ofs: Reg, len: Reg) {
        let i64_type = self.i64_type.unwrap();
        let rs = read_value!(self, rs);

        let ofs = ofs.storage.borrow().try_as_u64().unwrap();
        let len = len.storage.borrow().try_as_u64().unwrap();
        let left_shift = i64_type.const_int(64 - len - ofs, false);
        let right_shift = i64_type.const_int(64 - len, false);

        let chop_high = self.builder.build_left_shift(rs, left_shift, "");
        let result = self
            .builder
            .build_right_shift(chop_high, right_shift, true, "");
        store_result!(self, rd, result);
    }

    fn gen_load(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        let i64_type = self.i64_type.unwrap();

        let rs1 = read_value!(self, rs1);
        let mem_op = rs2.storage.borrow().try_as_u64().unwrap();
        let mem_op = MemOp::from_bits(mem_op).unwrap();
        let size: u64 = mem_op.get_size();
        let sign: bool = mem_op.get_sign();

        // calculate real address = offset + guest
        let offset = self.guest_vm.borrow().as_ptr() as u64;
        let offset = i64_type.const_int(offset, false);
        let addr = self.builder.build_int_add(rs1, offset, "");

        let ptr_type = match size {
            1 => self.context.i8_type(),
            2 => self.context.i16_type(),
            4 => self.context.i32_type(),
            8 => self.context.i64_type(),
            _ => unreachable!(),
        }
        .ptr_type(AddressSpace::Generic);

        let addr_ptr = self.builder.build_int_to_ptr(addr, ptr_type, "");
        let word = self.builder.build_load(addr_ptr, "").into_int_value();

        let result = if sign {
            self.builder.build_int_s_extend(word, i64_type, "")
        } else {
            self.builder.build_int_z_extend(word, i64_type, "")
        };

        store_result!(self, rd, result);
    }

    fn gen_add(&mut self, rd: Reg, rs1: Reg, rs2: Reg) {
        let rs1 = read_value!(self, rs1);
        let rs2 = read_value!(self, rs2);

        let result = self.builder.build_int_add(rs1, rs2, "");
        store_result!(self, rd, result);
    }

    fn gen_trap(&mut self, cause: Reg, val: Reg) {
        // store context before calling trap handler
        self.store_context();

        let cause = read_value!(self, cause);
        let val = read_value!(self, val);

        let i64_type = self.i64_type.unwrap();
        let handler_type = self.handler_type.unwrap();
        let handler_ptr_type = handler_type.ptr_type(AddressSpace::Generic);
        let handler = i64_type
            .const_int(self.handler as u64, false)
            .const_to_pointer(handler_ptr_type);

        self.builder
            .build_call(handler, &[cause.into(), val.into()], "");
    }
}
