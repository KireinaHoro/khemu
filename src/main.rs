use khemu::*;

use crate::runtime::*;
use khemu::guest::Disassembler;
use khemu::host::dump_ir::DumpIRHostContext;
use khemu::host::HostContext;

fn main() -> Result<(), String> {
    env_logger::init();

    let elf = read_elf()?;
    let mut disassembler = runtime::loader::load_program(&elf)?;
    let mut host = DumpIRHostContext::new();
    let result = disassembler.disas_block(4096);

    println!("IR generated:");
    host.emit_block(disassembler.get_ops());

    result
}
