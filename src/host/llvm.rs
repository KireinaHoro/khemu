use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::Module;
use inkwell::targets::{InitializationConfig, Target};
use inkwell::OptimizationLevel;

use std::fmt::{Display, Error, Formatter};

use crate::guest::*;
use crate::host::*;
use crate::ir::storage::*;
use crate::runtime::*;
use bitflags::_core::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

type GuestFunc = unsafe extern "C" fn() -> u64;

pub struct LLVMHostContext<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
}

pub enum LLVMHostStorage {}

impl Default for LLVMHostStorage {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Display for LLVMHostStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        unimplemented!()
    }
}

impl HostStorage for LLVMHostStorage {
    type HostContext = LLVMHostContext<'static>;

    fn try_as_u32(&self) -> Option<u32> {
        unimplemented!()
    }

    fn try_as_u64(&self) -> Option<u64> {
        unimplemented!()
    }

    fn try_as_f64(&self) -> Option<f64> {
        unimplemented!()
    }
}

impl HostBlock for JitFunction<'_, GuestFunc> {
    fn execute(&self) {
        unimplemented!()
    }
}

static mut LLVM_CTX: Option<LLVMHostContext> = None;

impl HostContext for LLVMHostContext<'static> {
    type StorageType = LLVMHostStorage;
    type BlockType = JitFunction<'static, GuestFunc>;

    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        tracking: &[Weak<KHVal<Self::StorageType>>],
        exception: Option<DisasException>,
    ) -> Self::BlockType {
        unimplemented!()
    }

    fn init(guest_map: GuestMap, handler: impl FnMut(u64, u64)) {
        // FIXME(jsteward): there should be a better way to do this (without leaking)
        let context = Box::new(Context::create());
        let context = Box::leak(context);

        let module = context.create_module("khemu");
        let execution_engine = module
            .create_jit_execution_engine(OptimizationLevel::None)
            .unwrap();

        unsafe {
            LLVM_CTX = Some(Self {
                context,
                module,
                builder: context.create_builder(),
                execution_engine,
            });
        }
    }

    fn get() -> &'static mut Self {
        unsafe { LLVM_CTX.as_mut().unwrap() }
    }

    fn make_label(&self) -> Self::StorageType {
        unimplemented!()
    }

    fn make_u32(&self, v: u32) -> Self::StorageType {
        unimplemented!()
    }

    fn make_u64(&self, v: u64) -> Self::StorageType {
        unimplemented!()
    }

    fn make_f64(&self, v: f64) -> Self::StorageType {
        unimplemented!()
    }

    fn make_named(&self, name: String) -> Self::StorageType {
        unimplemented!()
    }
}
