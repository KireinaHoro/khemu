// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::{Linkage, Module};
use inkwell::targets::{InitializationConfig, Target};
use inkwell::types::{FloatType, FunctionType, IntType};
use inkwell::values::{BasicValue, BasicValueEnum, FloatValue, GlobalValue, IntValue};
use inkwell::{AddressSpace, OptimizationLevel};

use log::*;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Error, Formatter};
use std::rc::{Rc, Weak};

use crate::guest::*;
use crate::host::*;
use crate::ir::op::*;
use crate::ir::storage::*;
use crate::runtime::*;
use bitflags::_core::cell::RefMut;

type GuestFunc = unsafe extern "C" fn();

/// Code generator context for the LLVM backend.
pub struct LLVMHostContext<'ctx> {
    context: &'ctx Context,
    // FIXME(jsteward): how to recycle these?
    modules: Vec<Module<'ctx>>,
    builder: Builder<'ctx>,
    execution_engine: Option<ExecutionEngine<'ctx>>,
    fn_type: Option<FunctionType<'ctx>>,
    i32_type: Option<IntType<'ctx>>,
    i64_type: Option<IntType<'ctx>>,
    f64_type: Option<FloatType<'ctx>>,
    handler_type: Option<FunctionType<'ctx>>,
    guest_vm: GuestMap,
    handler: TrapHandler,
    global_map: RefCell<HashMap<GlobalValue<'ctx>, Option<IntValue<'ctx>>>>,
    dump_reg_func: Option<JitFunction<'ctx, GuestFunc>>,
}

#[derive(Debug, PartialEq)]
/// LLVM Storage for IR registers.
///
/// Largely resembles the LLVM value categories.  Per the SSA semantics, assigned values cannot be
/// reassigned; a new one needs to be created instead.
pub enum LLVMHostStorage<'ctx> {
    /// A not-yet used register.
    Empty,
    /// A fixed register.
    Global(GlobalValue<'ctx>),
    /// Assigned int temporary value.
    IntV(IntValue<'ctx>),
    /// Assigned float temporary value.
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
    /// Execute the generated LLVM JIT function.
    unsafe fn execute(&self) {
        self.call()
    }
}

mod codegen;

static mut LLVM_CTX: Option<LLVMHostContext> = None;
static REG_INIT: u64 = 0;
static REG_INIT_FP: f64 = 0.0;

impl LLVMHostContext<'static> {
    fn store_context(&mut self) {
        // store cached values back into global
        for (k, v) in self.global_map.borrow_mut().iter_mut() {
            if let Some(iv) = v {
                debug!(
                    "Emitting store {} -> {}",
                    iv.print_to_string(),
                    k.get_name().to_str().unwrap()
                );
                self.builder.build_store(k.as_pointer_value(), *iv);
                *v = None;
            }
        }
    }

    fn dump_modules(&self) {
        for x in &self.modules {
            x.print_to_stderr();
        }
    }

    fn dump_reg(&mut self) {
        // check that all values are stored back into global
        for (k, v) in self.global_map.borrow().iter() {
            assert_eq!(*v, None, "found leaked value for global {:?}", k);
        }

        if let None = self.dump_reg_func {
            // build func
            let name = "trap_handler";
            self.push_block(name, true);

            let module = self.modules.last().unwrap();

            let printf = module.get_function("printf").unwrap_or_else(|| {
                let char_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
                let func_type = self
                    .context
                    .i32_type()
                    .fn_type(&[char_ptr_type.into()], true);
                module.add_function("printf", func_type, Some(Linkage::External))
            });

            let mut format_string = String::new();
            let mut tagged_args = Vec::new();
            for (k, _) in self.global_map.borrow().iter() {
                let name = String::from(k.get_name().to_str().unwrap());
                let val = self.builder.build_load(k.as_pointer_value(), "");

                tagged_args.push((name, val));
            }
            tagged_args.sort_by_key(|(n, _)| n.to_owned());
            let (names, mut args): (Vec<_>, VecDeque<_>) = tagged_args.into_iter().unzip();
            let names = names
                .iter()
                .map(|n| format!("{}=0x%016lx", n))
                .collect::<Vec<_>>();
            let mut format_string: String = names
                .chunks(4)
                .map(|c| c.join("\t"))
                .collect::<Vec<_>>()
                .join("\n");

            format_string.push('\n');

            args.push_front(BasicValueEnum::from(
                self.builder
                    .build_global_string_ptr(&format_string, "")
                    .as_pointer_value(),
            ));
            let args = args.into_iter().collect::<Vec<_>>();

            self.builder.build_call(printf, args.as_slice(), "");
            self.builder.build_return(None);

            unsafe {
                let f: JitFunction<GuestFunc> = self
                    .execution_engine
                    .as_ref()
                    .unwrap()
                    .get_function(name)
                    .unwrap();
                self.dump_reg_func = Some(f);
            }
        }

        unsafe { self.dump_reg_func.as_ref().unwrap().call() }
    }
}

impl HostContext for LLVMHostContext<'static> {
    /// Use LLVM values for IR register storage.
    type StorageType = LLVMHostStorage<'static>;
    /// Use LLVM JIT function as emitted block.
    type BlockType = JitFunction<'static, GuestFunc>;

    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        name: &str,
        tracking: &[Weak<KHVal<Self::StorageType>>],
        exception: Option<DisasException>,
    ) -> Self::BlockType {
        // consume TB
        for op in tb.ops.into_iter() {
            debug!("Emitting {}", op);
            self.dispatch(op);
        }

        // end block, insert return
        self.builder.build_return(None);

        unsafe {
            self.execution_engine
                .as_ref()
                .unwrap()
                .get_function(name)
                .expect("failed to get function from JIT engine")
        }
    }

    fn init(guest_vm: GuestMap, handler: TrapHandler) {
        // FIXME(jsteward): there should be a better way to do this (without leaking)
        let context = Box::new(Context::create());
        let context = Box::leak(context);

        unsafe {
            LLVM_CTX = Some(Self {
                context,
                modules: Vec::new(),
                builder: context.create_builder(),
                execution_engine: None,
                fn_type: None,
                i32_type: None,
                i64_type: None,
                f64_type: None,
                handler_type: None,
                guest_vm,
                handler,
                global_map: Default::default(),
                dump_reg_func: None,
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

            // create default module for guest fixed register initializer
            LLVM_CTX.as_mut().unwrap().push_block("default", false);
        }
    }

    fn get() -> &'static mut Self {
        unsafe { LLVM_CTX.as_mut().unwrap() }
    }

    fn push_block(&mut self, name: &str, create_func: bool) {
        self.modules.push(self.context.create_module(name));
        let module = self.modules.last().expect("failed to create module");

        if let None = self.execution_engine {
            self.execution_engine = Some(
                module
                    .create_jit_execution_engine(OptimizationLevel::None)
                    .expect("failed to create JIT engine"),
            );
        } else {
            self.execution_engine
                .as_ref()
                .unwrap()
                .add_module(module)
                .expect("failed to add new module to existing engine");
        }

        if create_func {
            let func = module.add_function(name, self.fn_type.unwrap(), None);

            let basic_block = self.context.append_basic_block(func, "entry");
            self.builder.position_at_end(basic_block);
        }
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
        let module = self.modules.last().expect("failed to get current module");
        let glb = match ty {
            ValueType::U32 => {
                let g = module.add_global(self.i32_type.unwrap(), None, name.as_ref());
                g.set_initializer(&self.i32_type.unwrap().const_int(REG_INIT, false));
                g
            }
            ValueType::U64 => {
                let g = module.add_global(self.i64_type.unwrap(), None, name.as_ref());
                g.set_initializer(&self.i64_type.unwrap().const_int(REG_INIT, false));
                g
            }
            ValueType::F64 => {
                let g = module.add_global(self.f64_type.unwrap(), None, name.as_ref());
                g.set_initializer(&self.f64_type.unwrap().const_float(REG_INIT_FP));
                g
            }
            _ => unreachable!(),
        };
        // record global
        self.global_map.borrow_mut().insert(glb, None);
        LLVMHostStorage::Global(glb)
    }

    fn handle_trap(&mut self) {
        info!("Dumping registers");
        self.dump_reg();

        info!("Dumping modules generated so far");
        self.dump_modules();
    }
}
