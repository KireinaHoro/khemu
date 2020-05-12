use super::ir::storage::*;
use crate::guest::{DisasException, TranslationBlock};
use crate::runtime::GuestMap;
use std::rc::Weak;

pub mod arm64;
pub mod dump_ir;

pub trait HostBlock {
    // run from start of block
    fn execute(&self, ctx: &mut impl HostContext);
}

pub trait HostContext {
    type StorageType: HostStorage;
    type BlockType: HostBlock;
    // emit host code block for an Op block
    // TB caching should be implemented here
    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        tracking: &[Weak<KHVal<Self::StorageType>>],
        exception: Option<DisasException>,
    ) -> Self::BlockType;

    fn new(guest_map: GuestMap, handler: impl FnMut(u64, u64)) -> Self;
}
