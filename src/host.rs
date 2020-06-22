// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::ir::storage::*;
use crate::guest::{DisasException, TranslationBlock};
use crate::runtime::{GuestMap, TrapHandler};
use std::rc::Weak;

/// The DumpIR dummy backend for IR printout.
pub mod dump_ir;
/// The LLVM backend.
pub mod llvm;

/// An emitted block that can be executed on the host.
pub trait HostBlock {
    /// Run from start of block.
    unsafe fn execute(&self);
}

/// Code generator functions to be invoked from the runtime.
pub trait HostContext {
    /// Storage type for the IR registers.
    type StorageType: HostStorage;
    /// Emitted block type.
    type BlockType: HostBlock;
    /// Emit host code block for a [TranslationBlock](../guest/struct.TranslationBlock.html).
    ///
    /// Parameters:
    /// - `tb`: translation block to emit.
    /// - `name`: name of the emitted block.
    /// - `exception`: reason of translation termination from frontend.
    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        name: &str,
        tracking: &[Weak<KHVal<Self::StorageType>>],
        exception: Option<DisasException>,
    ) -> Self::BlockType;

    /// Initialize the global backend context.
    fn init(guest_vm: GuestMap, handler: TrapHandler);
    /// Retrieve the global backend context.
    fn get() -> &'static mut Self;

    /// Allocate new block in backend for code generation.
    ///
    /// Must be called before any of the following for an emitted block.
    fn push_block(&mut self, name: &str, create_func: bool);

    /// Create a label value.
    fn make_label(&self) -> Self::StorageType;
    /// Create a `u32` value.  Backends may implement caching to avoid allocating duplicate values.
    fn make_u32(&self, v: u32) -> Self::StorageType;
    /// Create a `u64` value.  Backends may implement caching to avoid allocating duplicate values.
    fn make_u64(&self, v: u64) -> Self::StorageType;
    /// Create a `f64` value.  Backends may implement caching to avoid allocating duplicate values.
    fn make_f64(&self, v: f64) -> Self::StorageType;
    /// Create a named value for fixed registers.
    fn make_named(&self, name: String, ty: ValueType) -> Self::StorageType;

    /// Backend-specific routine for handling traps.
    ///
    /// Backend-irrelevant parts should go into `runtime::trap_handler`.
    fn handle_trap(&mut self);
}
