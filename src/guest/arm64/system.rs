// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;

pub fn disas_system<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let l = extract(insn, 21, 1) == 1;
    let op0 = extract(insn, 19, 2);
    let op1 = extract(insn, 16, 3);
    let crn = extract(insn, 12, 4);
    let crm = extract(insn, 8, 4);
    let op2 = extract(insn, 5, 3);
    let rt = extract(insn, 0, 5) as usize;

    if op0 == 0 {
        if l || rt != 31 {
            return unallocated(ctx, insn);
        }
        (match crn {
            2 => handle_hint,
            3 => handle_sync,
            4 => handle_msr_i,
            _ => return unallocated(ctx, insn),
        })(ctx, insn, op1, op2, crm)
    } else {
        handle_sys(ctx, insn, l, op0, op1, op2, crn, crm, rt)
    }
}

fn handle_hint<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    op1: u32,
    op2: u32,
    crm: u32,
) -> Result<(), DisasException> {
    let selector = crm << 3 | op2;

    if op1 != 3 {
        return unallocated(ctx, insn);
    }

    // TODO(jsteward) figure out proper behaviors for these instructions in EL0
    match selector {
        0 => {}     // nop
        3 => {}     // wfi
        1 => {}     // yield
        2 => {}     // wfe
        4 | 5 => {} // sev / sevl
        _ => {}     // pauth, etc. specified as nop-equivalent
    }

    Ok(())
}

fn handle_sync<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    op1: u32,
    op2: u32,
    crm: u32,
) -> Result<(), DisasException> {
    Err(DisasException::Unexpected(
        "handle_sync work in progress".to_owned(),
    ))
}

fn handle_msr_i<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    op1: u32,
    op2: u32,
    crm: u32,
) -> Result<(), DisasException> {
    Err(DisasException::Unexpected(
        "handle_msr_i work in progress".to_owned(),
    ))
}

fn handle_sys<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    isread: bool,
    op0: u32,
    op1: u32,
    op2: u32,
    crn: u32,
    crm: u32,
    rt: usize,
) -> Result<(), DisasException> {
    Err(DisasException::Unexpected(
        "handle_sys work in progress".to_owned(),
    ))
}
