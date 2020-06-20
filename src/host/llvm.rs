use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::Module;
use inkwell::targets::{InitializationConfig, Target};
use inkwell::types::{FloatType, FunctionType, IntType};
use inkwell::values::{BasicValue, FloatValue, GlobalValue, IntValue};
use inkwell::{AddressSpace, OptimizationLevel};

use log::*;
use std::collections::BTreeMap;
use std::fmt::{Display, Error, Formatter};
use std::rc::{Rc, Weak};

use crate::guest::*;
use crate::host::*;
use crate::ir::op::*;
use crate::ir::storage::*;
use crate::runtime::*;

type GuestFunc = unsafe extern "C" fn();

pub struct LLVMHostContext<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
    fn_type: Option<FunctionType<'ctx>>,
    i32_type: Option<IntType<'ctx>>,
    i64_type: Option<IntType<'ctx>>,
    f64_type: Option<FloatType<'ctx>>,
    handler_type: Option<FunctionType<'ctx>>,
    guest_vm: GuestMap,
    handler: TrapHandler,
}

#[derive(PartialEq)]
pub enum LLVMHostStorage<'ctx> {
    Empty,
    Global(GlobalValue<'ctx>),
    IntV(IntValue<'ctx>),
    FloatV(FloatValue<'ctx>),
}

impl Default for LLVMHostStorage<'_> {
    fn default() -> Self {
        // default unallocated storage for temporaries
        LLVMHostStorage::Empty
    }
}

impl Display for LLVMHostStorage<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            LLVMHostStorage::Empty => write!(f, "_"),
            LLVMHostStorage::Global(v) => write!(f, "{}", v.get_name().to_str().unwrap()),
            LLVMHostStorage::IntV(v) => write!(f, "{}", v.print_to_string()),
            LLVMHostStorage::FloatV(v) => write!(f, "{}", v.print_to_string()),
        }
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
    unsafe fn execute(&self) {
        self.call()
    }
}

mod codegen;

static mut LLVM_CTX: Option<LLVMHostContext> = None;

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

        // consume TB
        for op in tb.ops.into_iter() {
            debug!("Emitting {}", op);
            self.dispatch(op);
        }

        // TODO(jsteward) generate context store:
        // TODO(jsteward) check for guest registers, if not global, store to global

        // end block, insert return
        self.builder.build_return(None);

        self.module.print_to_stderr();

        unsafe { self.execution_engine.get_function(name.as_str()).unwrap() }
    }

    fn init(guest_vm: GuestMap, handler: TrapHandler) {
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
                i32_type: None,
                i64_type: None,
                f64_type: None,
                handler_type: None,
                guest_vm,
                handler,
            });

            LLVM_CTX.as_mut().unwrap().fn_type = Some(
                LLVM_CTX
                    .as_mut()
                    .unwrap()
                    .context
                    .void_type()
                    .fn_type(&[], false),
            );

            LLVM_CTX.as_mut().unwrap().i32_type =
                Some(LLVM_CTX.as_mut().unwrap().context.i32_type());
            LLVM_CTX.as_mut().unwrap().i64_type =
                Some(LLVM_CTX.as_mut().unwrap().context.i64_type());

            let i64_type = LLVM_CTX.as_mut().unwrap().i64_type.unwrap();

            LLVM_CTX.as_mut().unwrap().f64_type =
                Some(LLVM_CTX.as_mut().unwrap().context.f64_type());
            LLVM_CTX.as_mut().unwrap().handler_type = Some(
                LLVM_CTX
                    .as_mut()
                    .unwrap()
                    .context
                    .void_type()
                    .fn_type(&[i64_type.into(), i64_type.into()], false),
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
        LLVMHostStorage::IntV(self.i64_type.unwrap().const_int(v, false))
    }

    fn make_f64(&self, v: f64) -> Self::StorageType {
        unimplemented!()
    }

    fn make_named(&self, name: String, ty: ValueType) -> Self::StorageType {
        LLVMHostStorage::Global(match ty {
            ValueType::U32 => {
                let g = self
                    .module
                    .add_global(self.i32_type.unwrap(), None, name.as_ref());
                g.set_initializer(&self.i32_type.unwrap().const_int(0, false));
                g
            }
            ValueType::U64 => {
                let g = self
                    .module
                    .add_global(self.i64_type.unwrap(), None, name.as_ref());
                g.set_initializer(&self.i64_type.unwrap().const_int(0, false));
                g
            }
            ValueType::F64 => {
                let g = self
                    .module
                    .add_global(self.f64_type.unwrap(), None, name.as_ref());
                g.set_initializer(&self.f64_type.unwrap().const_float(0f64));
                g
            }
            _ => unreachable!(),
        })
    }
}
