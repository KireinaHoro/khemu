use super::ir::op::Op;

pub mod arm64;
pub mod dump_ir;

pub trait HostContext {
    // emit host code for an Op block
    fn emit_block(&mut self, ops: Vec<Op>);
    // get all emitted instructions and reset buffer
    fn get_insns(&mut self) -> Vec<u8>;
}
