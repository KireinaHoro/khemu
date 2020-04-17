pub mod arm64;

use crate::ir::op::Op;
use crate::ir::storage::*;
use std::rc::{Rc, Weak};

pub trait GuestContext<R: HostStorage>
where
    Self: Sized,
{
    type InsnType;
    // fetch a single guest instruction
    fn next_insn(&mut self) -> Option<Self::InsnType>;

    // allocate a new unassigned KHVal
    fn alloc_val(&mut self, ty: ValueType) -> Rc<KHVal<R>>;
    // allocate u64 immediate value
    fn alloc_u64(&mut self, v: u64) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::U64);
        *ret.storage.borrow_mut() = R::make_u64(v);
        ret
    }
    fn alloc_f64(&mut self, v: f64) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::F64);
        *ret.storage.borrow_mut() = R::make_f64(v);
        ret
    }
    // get tracking weak pointers of allocated KHVals
    fn get_tracking(&self) -> &[Weak<KHVal<R>>];
    // run housekeeping on tracking
    fn clean_tracking(&mut self);

    // push an Op into the buffer
    fn push_op(&mut self, op: Op<R>);
    // get all Ops in buffer and reset buffer
    fn get_ops(&mut self) -> Vec<Op<R>>;

    // run disassembly loop for a block of guest code
    fn disas_block(&mut self, block_size: u32) -> Result<(), String>;
}
