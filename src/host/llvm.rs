use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::Module;
use inkwell::targets::{InitializationConfig, Target};
use inkwell::types::FunctionType;
use inkwell::values::{FloatValue, GlobalValue, IntValue};
use inkwell::{AddressSpace, OptimizationLevel};

use std::collections::BTreeMap;
use std::fmt::{Display, Error, Formatter};
use std::rc::{Rc, Weak};

use crate::guest::*;
use crate::host::*;
use crate::ir::op::*;
use crate::ir::storage::*;
use crate::runtime::*;

type GuestFunc = unsafe extern "C" fn() -> u64;

pub struct LLVMHostContext<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
    fn_type: Option<FunctionType<'ctx>>,
}

pub enum LLVMHostStorage<'ctx> {
    Empty,
    Global(GlobalValue<'ctx>),
    IntV(IntValue<'ctx>),
    FloatV(FloatValue<'ctx>),
}

impl Default for LLVMHostStorage<'_> {
    fn default() -> Self {
        LLVMHostStorage::Empty
    }
}

impl Display for LLVMHostStorage<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "_")
    }
}

impl HostStorage for LLVMHostStorage<'static> {
    type HostContext = LLVMHostContext<'static>;

    fn try_as_u32(&self) -> Option<u32> {
        if let LLVMHostStorage::IntV(lv) = self {
            lv.get_zero_extended_constant().map(|x| x as u32)
        } else {
            None
        }
    }

    fn try_as_u64(&self) -> Option<u64> {
        if let LLVMHostStorage::IntV(lv) = self {
            lv.get_zero_extended_constant()
        } else {
            None
        }
    }

    fn try_as_f64(&self) -> Option<f64> {
        if let LLVMHostStorage::FloatV(lv) = self {
            lv.get_constant().map(|x| x.0)
        } else {
            None
        }
    }
}

impl HostBlock for JitFunction<'_, GuestFunc> {
    fn execute(&self) {
        unimplemented!()
    }
}

static mut LLVM_CTX: Option<LLVMHostContext> = None;

impl CodeGen<LLVMHostStorage<'static>> for LLVMHostContext<'static> {}

impl HostContext for LLVMHostContext<'static> {
    type StorageType = LLVMHostStorage<'static>;
    type BlockType = JitFunction<'static, GuestFunc>;

    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        tracking: &[Weak<KHVal<Self::StorageType>>],
        exception: Option<DisasException>,
    ) -> Self::BlockType {
        let name = format!("func_{}", tb.start_pc);
        let func = self
            .module
            .add_function(name.as_str(), self.fn_type.unwrap(), None);

        let basic_block = self.context.append_basic_block(func, "entry");
        self.builder.position_at_end(basic_block);

        // TODO(jsteward) generate context restore

        for op in tb.ops.into_iter() {
            self.dispatch_op(op);
        }

        // TODO(jsteward) generate context store

        unsafe { self.execution_engine.get_function(name.as_str()).unwrap() }
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
                fn_type: None,
            });

            LLVM_CTX.as_mut().unwrap().fn_type = Some(
                LLVM_CTX
                    .as_mut()
                    .unwrap()
                    .context
                    .void_type()
                    .fn_type(&[], false),
            );
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
        let ty = self.context.i64_type();

        LLVMHostStorage::IntV(ty.const_int(v, false))
    }

    fn make_f64(&self, v: f64) -> Self::StorageType {
        unimplemented!()
    }

    fn make_named(&self, name: String, ty: ValueType) -> Self::StorageType {
        let global = match ty {
            ValueType::U32 => self.module.add_global(
                self.context.i32_type(),
                Some(AddressSpace::Const),
                name.as_ref(),
            ),
            ValueType::U64 => self.module.add_global(
                self.context.i64_type(),
                Some(AddressSpace::Const),
                name.as_ref(),
            ),
            ValueType::F64 => self.module.add_global(
                self.context.f64_type(),
                Some(AddressSpace::Const),
                name.as_ref(),
            ),
            _ => unreachable!(),
        };

        LLVMHostStorage::Global(global)
    }
}
