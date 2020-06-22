// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

// This is a dummy host that simply dumps the IR without executing them

use crate::guest::{DisasException, TranslationBlock};
use crate::host::*;
use crate::ir::storage::*;
use crate::runtime::{GuestMap, TrapHandler};
use std::cell::RefCell;
use std::fmt::{Display, Error, Formatter};
use std::rc::Weak;

/// Dummy context for IR printout.
pub struct DumpIRHostContext {
    label_counter: RefCell<u64>,
}

#[derive(PartialEq)]
/// Dummy storage that only notes input constants for IR printing purposes.
///
/// No actual memory allocation for intermediate results will happen; they are denoted with the
/// `Unassigned` variant.
pub enum DumpIRHostStorage {
    Label(u64),
    ImmU32(u32),
    ImmU64(u64),
    ImmF64(f64),
    Named(String),
    Unassigned,
}

impl Default for DumpIRHostStorage {
    fn default() -> Self {
        DumpIRHostStorage::Unassigned
    }
}

impl Display for DumpIRHostStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            DumpIRHostStorage::Label(n) => write!(f, "L{}", n),
            DumpIRHostStorage::Named(name) => write!(f, "${}", name),
            DumpIRHostStorage::ImmF64(v) => write!(f, "#{}", v),
            DumpIRHostStorage::ImmU64(v) => write!(f, "#{:#x}", v),
            DumpIRHostStorage::ImmU32(v) => write!(f, "#{:#x}", v),
            // temporaries should use the value hash directly
            DumpIRHostStorage::Unassigned => Err(Error),
        }
    }
}

impl HostStorage for DumpIRHostStorage {
    type HostContext = DumpIRHostContext;

    fn try_as_u32(&self) -> Option<u32> {
        if let &DumpIRHostStorage::ImmU32(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn try_as_u64(&self) -> Option<u64> {
        if let &DumpIRHostStorage::ImmU64(v) = self {
            Some(v)
        } else {
            None
        }
    }

    fn try_as_f64(&self) -> Option<f64> {
        if let &DumpIRHostStorage::ImmF64(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl HostBlock for String {
    /// Dump the string contents of the dummy generated block.
    unsafe fn execute(&self) {
        print!("{}", self);
    }
}

static mut DUMP_IR_CTX: Option<DumpIRHostContext> = None;

impl HostContext for DumpIRHostContext {
    /// Use dummy storage for IR registers.
    type StorageType = DumpIRHostStorage;
    /// Use strings that hold IR printout as emitted blocks.
    type BlockType = String;

    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        _name: &str,
        _tracking: &[Weak<KHVal<Self::StorageType>>],
        _exception: Option<DisasException>,
    ) -> Self::BlockType {
        let mut ret = String::new();
        for op in tb.ops.into_iter() {
            ret += &format!("{}\n", op);
        }
        ret
    }

    fn init(_: GuestMap, _: TrapHandler) {
        unsafe {
            DUMP_IR_CTX = Some(Self {
                label_counter: RefCell::new(0),
            });
        }
    }

    fn get() -> &'static mut Self {
        unsafe { DUMP_IR_CTX.as_mut().unwrap() }
    }

    fn push_block(&mut self, name: &str, create_func: bool) {
        unimplemented!()
    }

    fn make_label(&self) -> Self::StorageType {
        let ret = DumpIRHostStorage::Label(*self.label_counter.borrow());
        *self.label_counter.borrow_mut() += 1;
        ret
    }

    fn make_u32(&self, v: u32) -> Self::StorageType {
        DumpIRHostStorage::ImmU32(v)
    }

    fn make_u64(&self, v: u64) -> Self::StorageType {
        DumpIRHostStorage::ImmU64(v)
    }

    fn make_f64(&self, v: f64) -> Self::StorageType {
        DumpIRHostStorage::ImmF64(v)
    }

    fn make_named(&self, name: String, ty: ValueType) -> Self::StorageType {
        DumpIRHostStorage::Named(name)
    }

    fn handle_trap(&mut self) {
        unimplemented!()
    }
}
