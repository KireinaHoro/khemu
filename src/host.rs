use super::ir::op::Op;
use super::ir::storage::*;
use crate::guest::TranslationBlock;

pub mod arm64;
pub mod dump_ir;

pub trait HostContext {
    type StorageType: HostStorage;
    // emit host code for an Op block
    // TB caching should be implemented here
    fn emit_block(&mut self, tb: TranslationBlock<Self::StorageType>);
    // get all emitted instructions and reset buffer
    fn get_insns(&mut self) -> Vec<u8>;
}
