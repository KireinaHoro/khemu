pub mod arm64;

use crate::ir::op::Op;

pub trait GuestContext<'a>
where
    Self: Sized,
{
    type InsnType;
    // fetch a single guest instruction
    fn next_insn(&mut self) -> Option<Self::InsnType>;
    // push an Op into the buffer
    fn push_op(&mut self, op: Op<'a>);
    // get all Ops in buffer and reset buffer
    fn get_ops(&mut self) -> Vec<Op<'a>>;
    // run disassembly loop for a block of guest code
    fn disas_block(&mut self) -> Result<(), String>;
}
