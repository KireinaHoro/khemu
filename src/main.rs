use khemu::*;

use crate::runtime::*;
use khemu::host::llvm::LLVMHostContext;

fn main() -> Result<(), String> {
    env_logger::init();

    do_work::<LLVMHostContext>()
}
