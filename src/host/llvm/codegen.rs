use super::*;

type Reg = Rc<KHVal<LLVMHostStorage<'static>>>;

macro_rules! read_value {
    ($self:expr, $rs:expr) => {
        match *$rs.storage.borrow() {
            LLVMHostStorage::Empty => panic!("trying to use empty value"),
            LLVMHostStorage::Global(v) => $self
                .builder
                .build_load(v.as_pointer_value(), "")
                .into_int_value(),
            LLVMHostStorage::IntV(v) => v,
            _ => panic!("not implemented"),
        }
    }
}

macro_rules! store_result {
    ($self:expr, $rd:expr, $result:expr) => {
        let mut rd_storage = $rd.storage.borrow_mut();
        match *rd_storage {
            LLVMHostStorage::Empty => *rd_storage = LLVMHostStorage::IntV($result),
            LLVMHostStorage::Global(v) => {
                $self.builder.build_store(v.as_pointer_value(), $result);
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
}
