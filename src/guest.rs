pub mod arm64;

use crate::ir::op::Op;
use crate::ir::storage::*;
use std::rc::{Rc, Weak};

pub trait Disassembler<R: HostStorage> {
    // run disassembly loop for a block of guest code
    fn disas_block(&mut self, block_size: u32) -> Result<(), String>;

    // push an Op into the buffer
    fn push_op(&mut self, op: Op<R>);
    // get all Ops in buffer and reset buffer
    fn get_ops(&mut self) -> Vec<Op<R>>;
}

pub trait DisasContext<R: HostStorage>
where
    Self: Sized,
{
    type InsnType;
    // fetch a single guest instruction
    fn next_insn(&mut self) -> Option<Self::InsnType>;
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

    // get tracking weak pointers of allocated KHVals
    fn get_tracking(&self) -> &[Weak<KHVal<R>>];
    // run housekeeping on tracking
    fn clean_tracking(&mut self);
}
