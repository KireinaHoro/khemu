use super::ir::op::Op;
use super::ir::storage::*;

pub mod arm64;
pub mod dump_ir;

pub trait HostContext {
    type StorageType: HostStorage;
    // emit host code for an Op block
    fn emit_block(&mut self, ops: Vec<Op<Self::StorageType>>);
    // get all emitted instructions and reset buffer
    fn get_insns(&mut self) -> Vec<u8>;
}
