use khemu::*;

use crate::guest::arm64::Arm64GuestContext;
use crate::guest::GuestContext;
use crate::host::dump_ir::{DumpIRHostContext, DumpIRHostStorage};
use khemu::host::HostContext;

// codegen basic functionality test
// disassemble hard-coded code block and print IR
const ARM64_EXAMPLE: &'static [u8] = &[
    0xfd, 0x7b, 0xbf, 0xa9, 0x1f, 0x08, 0x00, 0x71, 0xfd, 0x03, 0x00, 0x91, 0x40, 0x01, 0x00, 0x54,
    0x80, 0x00, 0x00, 0xd0, 0x00, 0x03, 0x00, 0xd0, 0x22, 0x00, 0x40, 0xf9, 0x61, 0x80, 0x14, 0x91,
    0x00, 0xdc, 0x47, 0xf9, 0x00, 0x00, 0x40, 0xf9, 0xee, 0xff, 0xff, 0x97, 0x20, 0x00, 0x80, 0x52,
];

fn main() -> Result<(), String> {
    let mut ctx = CodeGenContext {
        guest: Arm64GuestContext::<DumpIRHostStorage>::new(ARM64_EXAMPLE),
        host: DumpIRHostContext::new(),
    };
    let result = ctx.guest.disas_block(4096);

    println!("IR generated:");
    // TODO(jsteward) insert IR optimization passes here
    ctx.host.emit_block(ctx.guest.get_ops());

    result
}
