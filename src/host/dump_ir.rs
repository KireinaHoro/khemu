// This is a dummy host that simply dumps the IR without executing them

use crate::host::*;
use crate::ir::op::*;
use crate::ir::storage::HostStorage;
use std::fmt::{Display, Error, Formatter};

pub struct DumpIRHostContext {
    content: String,
}

impl DumpIRHostContext {
    pub fn new() -> Self {
        Self {
            content: String::new(),
        }
    }
}

// dummy interface, no real allocation
pub enum DumpIRHostStorage {
    ImmU64(u64),
    ImmF64(f64),
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
            DumpIRHostStorage::Unassigned => write!(f, "_"),
            DumpIRHostStorage::ImmF64(v) => write!(f, "{}", v),
            DumpIRHostStorage::ImmU64(v) => write!(f, "{}", v),
        }
    }
}

impl HostStorage for DumpIRHostStorage {
    fn make_u64(v: u64) -> Self {
        DumpIRHostStorage::ImmU64(v)
    }

    fn make_f64(v: f64) -> Self {
        DumpIRHostStorage::ImmF64(v)
    }
}

impl HostContext for DumpIRHostContext {
    type StorageType = DumpIRHostStorage;

    fn emit_block(&mut self, ops: Vec<Op<DumpIRHostStorage>>) {
        for op in ops.into_iter() {
            println!("{}", op)
        }
    }

    fn get_insns(&mut self) -> Vec<u8> {
        unimplemented!("DumpIR will not generate instructions")
    }
}
