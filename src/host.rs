// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::ir::storage::*;
use crate::guest::{DisasException, TranslationBlock};
use crate::runtime::{GuestMap, TrapHandler};
use std::rc::Weak;

pub mod dump_ir;
pub mod llvm;

pub trait HostBlock {
    // run from start of block
    unsafe fn execute(&self);
}

pub trait HostContext {
    type StorageType: HostStorage;
    type BlockType: HostBlock;
    // emit host code block for an Op block
    // TB caching should be implemented here
    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        name: &str,
        tracking: &[Weak<KHVal<Self::StorageType>>],
        exception: Option<DisasException>,
    ) -> Self::BlockType;

    fn init(guest_vm: GuestMap, handler: TrapHandler);
    fn get() -> &'static mut Self;

    // new block for codegen
    fn push_block(&mut self, name: &str, create_func: bool);

    // value creators
    fn make_label(&self) -> Self::StorageType;
    fn make_u32(&self, v: u32) -> Self::StorageType;
    fn make_u64(&self, v: u64) -> Self::StorageType;
    fn make_f64(&self, v: f64) -> Self::StorageType;
    fn make_named(&self, name: String, ty: ValueType) -> Self::StorageType;

    // host-specific routine for handling traps
    // host-irrelevant parts should go into `runtime::trap_handler`
    fn handle_trap(&mut self);
}
