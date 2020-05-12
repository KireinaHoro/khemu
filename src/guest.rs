pub mod arm64;

use crate::ir::op::Op;
use crate::ir::storage::*;
use crate::runtime::GuestMap;
use bitflags::_core::fmt::{Error, Formatter};
use std::fmt::Display;
use std::rc::{Rc, Weak};

// exceptions during disassembly that terminated
// the current translation block to start a new one
pub enum DisasException {
    // direct continue to next instruction
    // possible cases: jump target (forcing start of a new block), size limit exceeded
    Continue(usize), // usize: next pc
    // branch (conditional / unconditional)
    // possible cases: direct branch (and link), conditional branch
    // parameters denote statically-resolvable target
    Branch(Option<usize>, Option<usize>), // Option<usize>: target pc
    // unexpected error
    Unexpected(String), // String: cause
}

impl Display for DisasException {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            DisasException::Continue(d) => write!(f, "TB size exceeded, next instr: {:#x}", d),
            DisasException::Branch(d, a) => {
                let df = if let Some(a) = d {
                    format!("{:#x}", a)
                } else {
                    "unknown".to_owned()
                };
                let af = if let Some(a) = a {
                    format!("{:#x}", a)
                } else {
                    "unknown".to_owned()
                };
                write!(f, "branch: direct {}; aux {}", df, af)
            }
            DisasException::Unexpected(s) => write!(f, "unexpected: {}", s),
        }
    }
}

pub struct TranslationBlock<R: HostStorage> {
    pub start_pc: usize,
    pub ops: Vec<Op<R>>,
    // index of LOOKUP_TB trap in ops
    pub direct_chain_idx: Option<usize>, // taken branch
    pub aux_chain_idx: Option<usize>,    // not taken branch
}

pub trait Disassembler<R: HostStorage> {
    // run disassembly loop for a block of guest code
    fn disas_block(&mut self, start_pos: usize, tb_size: usize) -> DisasException;

    // get the newly-generated TB
    fn get_tb(&mut self) -> TranslationBlock<R>;

    // get memory map for execution use
    fn get_guest_map(&self) -> GuestMap;

    // get tracking weak pointers of allocated KHVals
    fn get_tracking(&self) -> &[Weak<KHVal<R>>];
    // run housekeeping on tracking
    fn clean_tracking(&mut self);
}

pub trait DisasContext<R: HostStorage>: Disassembler<R>
where
    Self: Sized,
{
    // read PC of last fetched instruction
    fn curr_pc(&self) -> usize;
    // PC of upcoming instruction
    fn next_pc(&self) -> usize;

    // allocate a new unassigned KHVal
    fn alloc_val(&mut self, ty: ValueType) -> Rc<KHVal<R>>;
    // allocate a new label
    fn alloc_label(&mut self) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::Label);
        *ret.storage.borrow_mut() = R::make_label();
        ret
    }
    // allocate u32 immediate value
    fn alloc_u32(&mut self, v: u32) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::U32);
        *ret.storage.borrow_mut() = R::make_u32(v);
        ret
    }
    // allocate u64 immediate value
    fn alloc_u64(&mut self, v: u64) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::U64);
        *ret.storage.borrow_mut() = R::make_u64(v);
        ret
    }
    // allocate f64 immediate value
    fn alloc_f64(&mut self, v: f64) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::F64);
        *ret.storage.borrow_mut() = R::make_f64(v);
        ret
    }

    // push an Op into the current translation block
    // should queue the Op pending for next translation block whenever there is an exception
    fn push_op(&mut self, op: Op<R>);
}
