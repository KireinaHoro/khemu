extern crate paste;

use crate::guest::*;
use crate::ir::op::*;
use crate::ir::storage::*;
use crate::util::*;

mod facility;

pub type InsnType = u32;

// disassembly facility
pub struct Arm64GuestContext<'a, R: HostStorage> {
    code: &'a [u8],
    disas_pos: usize,
    // 32 general-purpose registers
    xreg: Vec<Rc<KHVal<R>>>,
    ops: Vec<Op<R>>,
    // tracking Weak for allocated values
    tracking: Vec<Weak<KHVal<R>>>,
}

impl<'a, R: HostStorage> Arm64GuestContext<'a, R> {
    pub fn new(code: &'a [u8]) -> Self {
        Self {
            code,
            disas_pos: 0,
            xreg: repeat_with(|| Rc::new(KHVal::new(ValueType::U64)))
                .take(32)
                .collect(),
            ops: Vec::new(),
            tracking: Vec::new(),
        }
    }
}

impl<'a, R: HostStorage> GuestContext<R> for Arm64GuestContext<'a, R> {
    type InsnType = InsnType;

    fn next_insn(&mut self) -> Option<Self::InsnType> {
        // we only support 4-byte RISC for now
        let addr = self.disas_pos;
        if addr >= self.code.len() {
            None
        } else {
            let ret: &[u8] = &self.code[addr..addr + 4];
            self.disas_pos = addr + 4;
            Some(
                ret.iter()
                    .enumerate()
                    .fold(0, |c, (i, &v)| c | ((v as Self::InsnType) << (i * 8))),
            )
        }
    }

    fn alloc_val(&mut self, ty: ValueType) -> Rc<KHVal<R>> {
        let ret = Rc::new(KHVal::new(ty));
        self.tracking.push(Rc::downgrade(&ret));
        ret
    }

    fn get_tracking(&self) -> &[Weak<KHVal<R>>] {
        self.tracking.as_slice()
    }

    fn clean_tracking(&mut self) {
        self.tracking.retain(|x| x.weak_count() > 0);
    }

    fn push_op(&mut self, op: Op<R>) {
        self.ops.push(op)
    }

    fn get_ops(&mut self) -> Vec<Op<R>> {
        let mut ret = Vec::new();
        std::mem::swap(&mut ret, &mut self.ops);
        ret
    }

    fn disas_block(&mut self, block_size: u32) -> Result<(), String> {
        let mut i = 0;
        loop {
            if i > block_size {
                // block size reached
                return Ok(());
            }
            if let Some(insn) = self.next_insn() {
                disas_single(self, insn)?;
            } else {
                // no more instruction left
                return Ok(());
            }
            i += 1;
        }
    }
}

// AArch64 Opcodes
#[allow(dead_code, unused)]
fn unallocated<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), String> {
    // TODO(jsteward) we should handle this as SIGILL
    Err(format!("unallocated opcode; instruction: 0x{:08x}", insn))
}

macro_rules! disas_category {
    ( $name:ident, $start:expr, $len:expr, $( [ $($opc:expr),* ], $handler:ident ),* ) => {
        paste::item! {
            fn [< disas_ $name >]<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, insn: InsnType) -> Result<(), String> {
                (match extract(insn, $start, $len) {
                    $(
                        $($opc)|* => paste::expr! { [< disas_ $handler >] },
                    )*
                    _ => unallocated
                })(ctx, insn)
            }
        }
    };
}

macro_rules! disas_subcategory {
    ( $name:ident, $start:expr, $len:expr, $( [ $($opc:expr),* ], $handler:ident ),* ) => {
        mod $name;
        paste::item! {
            fn [< disas_ $name >]<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, insn: InsnType) -> Result<(), String> {
                (match extract(insn, $start, $len) {
                    $(
                        $($opc)|* => paste::expr! { $name :: [< disas_ $handler >] },
                    )*
                    _ => unallocated
                })(ctx, insn)
            }
        }
    };
}

macro_rules! disas_stub {
    ( $($handler:ident),* ) => {
        $(
            paste::item! {
                #[allow(dead_code, unused)]
                pub fn [< disas_ $handler >]<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, insn: InsnType) -> Result<(), String> {
                    Err(format!("{} not implemented", stringify!($handler)))
                }
            }
        )*
    };
}

#[rustfmt::skip]
disas_category!(single, 25, 4,
    [0x2], sve,
    [0x8, 0x9], data_proc_imm,
    [0x5, 0xd], data_proc_reg,
    [0x7, 0xf], data_proc_simd_fp,
    [0xa, 0xb], b_exc_sys,
    [0x4, 0x6, 0xc, 0xe], ldst
);
disas_stub![sve];

#[rustfmt::skip]
disas_subcategory!(data_proc_imm, 23, 6,
    [0x20, 0x21], pc_rel_addr,
    [0x22, 0x23], add_sub_imm,
    [0x24], logic_imm,
    [0x25], movw_imm,
    [0x26], bitfield,
    [0x27], extract
);

mod data_proc_reg;
use data_proc_reg::disas_data_proc_reg;

#[rustfmt::skip]
disas_subcategory!(ldst, 24, 6,
    [0x08], ldst_excl,
    [0x18], ld_lit,
    [0x28, 0x29, 0x2c, 0x2d], ldst_pair,
    [0x38, 0x39, 0x3c, 0x3d], ldst_reg,
    [0x0c], ldst_multiple_struct,
    [0x0d], ldst_single_struct,
    [0x19], ldst_ldapr_stlr
);

mod data_proc_simd_fp;
use data_proc_simd_fp::disas_data_proc_simd_fp;
use std::iter::repeat_with;
use std::rc::{Rc, Weak};

#[rustfmt::skip]
disas_subcategory!(b_exc_sys, 25, 7,
    [0x0a, 0x0b, 0x4a, 0x4b], uncond_b_imm,
    [0x1a, 0x5a], comp_b_imm,
    [0x1b, 0x5b], test_b_imm,
    [0x2a], cond_b_imm,
    [0x6a], exc_sys,
    [0x6b], uncond_b_reg
);
