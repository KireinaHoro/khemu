extern crate paste;

use crate::guest::*;
use crate::ir::op::*;
use crate::ir::storage::*;
use crate::runtime::*;
use crate::util::*;
use std::collections::{BTreeMap, HashMap};
use std::iter::*;
use std::rc::{Rc, Weak};

use log::*;

pub type InsnType = u32;

// disassembly facility
pub struct Arm64GuestContext<R: HostStorage> {
    map: GuestMap,            // memory map for guest
    disas_pos: Option<usize>, // addr for next instruction to be disassembled
    // 32 general-purpose registers
    xreg: Vec<Rc<KHVal<R>>>,
    // Negative, Zero, Carry, Overflow
    nf: Rc<KHVal<R>>,
    zf: Rc<KHVal<R>>,
    cf: Rc<KHVal<R>>,
    vf: Rc<KHVal<R>>,
    // emulated PC
    pc: Rc<KHVal<R>>,
    // TB book-keeping
    start_pc: Option<usize>,
    // emitted IR operations in current TB
    ops: Vec<Op<R>>,
    // chaining points
    direct_chain_idx: Option<usize>,
    aux_chain_idx: Option<usize>,
    // tracking Weak for allocated values
    tracking: Vec<Weak<KHVal<R>>>,
    u32_cache: HashMap<u32, Rc<KHVal<R>>>,
    u64_cache: HashMap<u64, Rc<KHVal<R>>>,
}

impl<R: HostStorage> Arm64GuestContext<R> {
    pub fn new(map: GuestMap) -> Self {
        Self {
            map,
            disas_pos: None,
            xreg: (0..32)
                .map(|i| {
                    Rc::new(KHVal::named(
                        if i == 31 {
                            "sp".to_owned()
                        } else {
                            format!("x{}", i)
                        },
                        ValueType::U64,
                    ))
                })
                .collect(),
            // use 32bit to simplify calculation when reading NZCV as a whole
            nf: Rc::new(KHVal::named("nf".to_owned(), ValueType::U32)),
            zf: Rc::new(KHVal::named("zf".to_owned(), ValueType::U32)),
            cf: Rc::new(KHVal::named("cf".to_owned(), ValueType::U32)),
            vf: Rc::new(KHVal::named("vf".to_owned(), ValueType::U32)),
            // 64bit simulated PC
            pc: Rc::new(KHVal::named("pc".to_owned(), ValueType::U64)),
            start_pc: None,
            ops: Vec::new(),
            direct_chain_idx: None,
            aux_chain_idx: None,
            tracking: Vec::new(),
            u32_cache: HashMap::new(),
            u64_cache: HashMap::new(),
        }
    }

    pub fn reg(&mut self, r: usize) -> Rc<KHVal<R>> {
        assert!(r < 32);
        if r == 31 {
            self.alloc_u64(0)
        } else {
            Rc::clone(&self.xreg[r])
        }
    }

    pub fn reg_sp(&self, r: usize) -> Rc<KHVal<R>> {
        assert!(r < 32);
        Rc::clone(&self.xreg[r])
    }

    pub fn set_direct_chain(&mut self) {
        if let Some(_) = self.direct_chain_idx {
            panic!("direct chain set twice in a single translation block")
        }
        self.direct_chain_idx = Some(self.ops.len() - 1);
    }

    pub fn set_aux_chain(&mut self) {
        if let Some(_) = self.aux_chain_idx {
            panic!("aux chain set twice in a single translation block")
        }
        self.aux_chain_idx = Some(self.ops.len() - 1);
    }

    // clean state for next translation block
    pub fn clean_state(&mut self) {
        self.disas_pos = None;
        self.start_pc = None;
        self.direct_chain_idx = None;
        self.aux_chain_idx = None;
    }
}

impl<R: HostStorage> DisasContext<R> for Arm64GuestContext<R> {
    type InsnType = InsnType;

    fn next_insn(&mut self) -> Self::InsnType {
        // normal aarch64 (not Thumb) has 4-byte instructions
        let addr = self.disas_pos.unwrap();
        let (k, v) = self.map.get_region(addr);
        let offset = addr - k;
        if offset >= v.len() {
            panic!("address {:#x} out of range", addr);
        } else {
            let ret: &[u8] = &v[offset..offset + 4];
            self.disas_pos = Some(addr + 4);
            ret.iter()
                .enumerate()
                .fold(0, |c, (i, &v)| c | ((v as Self::InsnType) << (i * 8)))
        }
    }

    fn curr_pc(&self) -> usize {
        self.disas_pos.unwrap() - 4
    }

    fn next_pc(&self) -> usize {
        self.disas_pos.unwrap()
    }

    fn alloc_val(&mut self, ty: ValueType) -> Rc<KHVal<R>> {
        let ret = Rc::new(KHVal::new(ty));
        self.tracking.push(Rc::downgrade(&ret));
        ret
    }

    // override the default implementation to cache smaller immediate values
    fn alloc_u32(&mut self, v: u32) -> Rc<KHVal<R>> {
        match self.u32_cache.get(&v) {
            None => {
                let ret = Rc::new(KHVal::u32(v));
                self.tracking.push(Rc::downgrade(&ret));
                self.u32_cache.insert(v, Rc::clone(&ret));
                ret
            }
            Some(r) => Rc::clone(r),
        }
    }

    // override the default implementation to cache smaller immediate values
    fn alloc_u64(&mut self, v: u64) -> Rc<KHVal<R>> {
        match self.u64_cache.get(&v) {
            None => {
                let ret = Rc::new(KHVal::u64(v));
                self.tracking.push(Rc::downgrade(&ret));
                self.u64_cache.insert(v, Rc::clone(&ret));
                ret
            }
            Some(r) => Rc::clone(r),
        }
    }

    // override the default implementation to reduce substitution overhead
    fn alloc_f64(&mut self, v: f64) -> Rc<KHVal<R>> {
        let ret = Rc::new(KHVal::f64(v));
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
}

impl<R: HostStorage> Disassembler<R> for Arm64GuestContext<R> {
    fn disas_block(&mut self, start_pos: usize, tb_size: usize) -> DisasException {
        self.start_pc = Some(start_pos);
        self.disas_pos = Some(start_pos);
        loop {
            if self.ops.len() >= tb_size {
                let next = self.alloc_u64(self.next_pc() as u64);
                Op::push_trap(self, TrapOp::LOOKUP_TB, &next);
                return DisasException::LimitReached(self.next_pc());
            } else {
                let insn = self.next_insn();
                if let Err(e) = disas_single(self, insn) {
                    return e;
                }
            }
        }
    }

    fn get_tb(&mut self) -> TranslationBlock<R> {
        let mut ret = Vec::new();
        std::mem::swap(&mut ret, &mut self.ops);

        let ret = TranslationBlock {
            start_pc: self.start_pc.unwrap(),
            ops: ret,
            direct_chain_idx: self.direct_chain_idx,
            aux_chain_idx: self.aux_chain_idx,
        };
        self.clean_state();

        ret
    }

    fn get_guest_map(&self) -> GuestMap {
        Rc::clone(&self.map)
    }
}

// AArch64 Opcodes
fn unallocated<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    // Emit trap to runtime with UNDEF cause and emulated PC value
    Op::push_trap(ctx, TrapOp::UNDEF_OPCODE, &Rc::clone(&ctx.pc));

    Ok(())
}

macro_rules! disas_category {
    ( $name:ident, $start:expr, $len:expr, $( [ $($opc:expr),* ], $handler:ident ),* ) => {
        paste::item! {
            fn [< disas_ $name >]<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, insn: InsnType) -> Result<(), DisasException> {
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
        paste::item! {
            fn [< disas_ $name >]<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, insn: InsnType) -> Result<(), DisasException> {
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
                pub fn [< disas_ $handler >]<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, insn: InsnType) -> Result<(), DisasException> {
                    Err(DisasException::Unexpected(format!("insn 0x{:0x}: {} not implemented", insn, stringify!($handler))))
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

// declare the submodules
// IntelliJ Rust will not recognize if these are in macro definition
mod b_exc_sys;
mod data_proc_imm;
mod data_proc_reg;
mod data_proc_simd_fp;
mod facility;
mod ldst;
mod sve;
