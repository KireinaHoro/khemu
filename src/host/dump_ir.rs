// This is a dummy host that simply dumps the IR without executing them

use crate::host::*;
use crate::ir::op::*;

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

impl HostContext for DumpIRHostContext {
    fn emit_block(&mut self, ops: Vec<Op>) {
        for op in ops.into_iter() {
            println!("{}", op)
        }
    }

    fn get_insns(&mut self) -> Vec<u8> {
        unimplemented!()
    }
}
