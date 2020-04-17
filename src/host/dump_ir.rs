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
#[derive(Default)]
pub struct DumpIRHostStorage {}

impl Display for DumpIRHostStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "_")
    }
}

impl HostStorage for DumpIRHostStorage {}

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
