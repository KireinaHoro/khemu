use super::ir::storage::*;
use crate::guest::{DisasException, TranslationBlock};
use std::rc::Weak;

pub mod arm64;
pub mod dump_ir;

pub trait HostContext {
    type StorageType: HostStorage;
    // emit host code for an Op block
    // TB caching should be implemented here
    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        tracking: &[Weak<KHVal<Self::StorageType>>],
        exception: Option<DisasException>,
    );
    // get all emitted instructions and reset buffer
    fn get_insns(&mut self) -> Vec<u8>;
}
