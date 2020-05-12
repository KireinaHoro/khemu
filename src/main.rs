use khemu::*;

use crate::runtime::*;
use khemu::host::dump_ir::DumpIRHostContext;

fn main() -> Result<(), String> {
    env_logger::init();

    do_work::<DumpIRHostContext>()
}
