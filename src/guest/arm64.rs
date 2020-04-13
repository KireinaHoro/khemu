extern crate paste;

use crate::guest::*;
use crate::util::*;
use crate::*;

mod facility;

pub type RegType = u64;
pub type InsnType = u32;

// disassembly facility
pub struct Arm64GuestContext<'a> {
    code: &'a [u8],
    disas_pos: usize,
    // TODO(jsteward) properly allocate registers
    xreg: [ir::KHVal<RegType>; 32],
}

impl<'a> Arm64GuestContext<'a> {
    pub fn new(code: &'a [u8]) -> Self {
        Self {
            code,
            disas_pos: 0,
            xreg: Default::default(),
        }
    }
}

impl<'a> GuestContext for Arm64GuestContext<'a> {
    type InsnType = InsnType;
    type RegType = RegType;

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

    fn disas_loop<HT>(ctx: &mut EmuContext<Self, HT>) -> Result<(), String>
    where
        HT: host::HostContext<RegType = RegType>,
    {
        loop {
            if let Some(insn) = ctx.guest.next_insn() {
                disas_single(ctx, insn)?;
            } else {
                println!("disas completed");
                break;
            }
        }

        Ok(())
    }
}

// AArch64 Opcodes
fn unallocated<HT>(
    ctx: &mut EmuContext<Arm64GuestContext, HT>,
    insn: InsnType,
) -> Result<(), String>
where
    HT: host::HostContext<RegType = RegType>,
{
    Err(format!("unallocated opcode; instruction: 0x{:08x}", insn))
}

macro_rules! disas_category {
    ( $name:ident, $start:expr, $len:expr, $( [ $($opc:expr),* ], $handler:ident ),* ) => {
        paste::item! {
            fn [< disas_ $name >]<HT>(ctx: &mut EmuContext<Arm64GuestContext, HT>, insn: InsnType) -> Result<(), String>
            where
                HT: host::HostContext<RegType = RegType>
            {
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
            fn [< disas_ $name >]<HT>(ctx: &mut EmuContext<Arm64GuestContext, HT>, insn: InsnType) -> Result<(), String>
            where
                HT: host::HostContext<RegType = RegType>
            {
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
                #[allow(dead_code)]
                pub fn [< disas_ $handler >]<HT>(ctx: &mut EmuContext<Arm64GuestContext, HT>, insn: InsnType) -> Result<(), String>
                where
                    HT: host::HostContext<RegType = RegType>
                {
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

#[rustfmt::skip]
disas_subcategory!(b_exc_sys, 25, 7,
    [0x0a, 0x0b, 0x4a, 0x4b], uncond_b_imm,
    [0x1a, 0x5a], comp_b_imm,
    [0x1b, 0x5b], test_b_imm,
    [0x2a], cond_b_imm,
    [0x6a], exc_sys,
    [0x6b], uncond_b_reg
);
