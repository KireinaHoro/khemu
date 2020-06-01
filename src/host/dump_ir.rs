// This is a dummy host that simply dumps the IR without executing them

use crate::guest::{DisasException, TranslationBlock};
use crate::host::*;
use crate::ir::storage::*;
use crate::runtime::GuestMap;
use std::fmt::{Display, Error, Formatter};
use std::rc::Weak;

pub struct DumpIRHostContext {}

// dummy interface, no real allocation
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
    fn execute(&self) {
        print!("{}", self);
    }
}

static mut DUMP_IR_CTX: Option<DumpIRHostContext> = None;

impl HostContext for DumpIRHostContext {
    type StorageType = DumpIRHostStorage;
    type BlockType = String;

    fn emit_block(
        &mut self,
        tb: TranslationBlock<Self::StorageType>,
        _tracking: &[Weak<KHVal<Self::StorageType>>],
        _exception: Option<DisasException>,
    ) -> Self::BlockType {
        let mut ret = String::new();
        for op in tb.ops.into_iter() {
            ret += &format!("{}\n", op);
        }
        ret
    }

    fn init(_: GuestMap, handler: impl FnMut(u64, u64)) {
        unsafe {
            DUMP_IR_CTX = Some(Self {});
        }
    }

    fn get() -> &'static mut Self {
        unsafe { DUMP_IR_CTX.as_mut().unwrap() }
    }

    fn make_label(&self) -> Self::StorageType {
        static mut COUNTER: u64 = 0;
        let ret;
        unsafe {
            ret = DumpIRHostStorage::Label(COUNTER);
            COUNTER += 1;
        }
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
}
