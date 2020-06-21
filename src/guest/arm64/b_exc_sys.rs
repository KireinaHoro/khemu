// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;
use crate::guest::arm64::facility::*;

pub fn disas_exc_sys<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    (if extract(insn, 24, 1) == 1 {
        if extract(insn, 22, 2) == 0 {
            super::system::disas_system
        } else {
            super::unallocated
        }
    } else {
        disas_exc
    })(ctx, insn)
}

pub fn disas_cond_b_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    if extract(insn, 4, 1) == 1 || extract(insn, 24, 1) == 1 {
        return unallocated(ctx, insn);
    }

    let addr = (ctx.curr_pc() as i64 + sextract(insn as i64, 5, 19) * 4) as usize;
    let cond = extract(insn, 0, 4);
    let addr_val = ctx.alloc_u64(addr as u64);

    if cond < 0xe {
        // genuine conditional branches
        let label = ctx.alloc_label();
        do_test_jump_cc(ctx, cond, &label);
        let next_pc = ctx.alloc_u64(ctx.next_pc() as u64);
        do_end_tb_to_addr(ctx, &next_pc, true); // branch not taken
        Op::push_setlbl(ctx, &label);
        do_end_tb_to_addr(ctx, &addr_val, false); // branch taken

        Err(DisasException::Branch(Some(addr), Some(ctx.next_pc())))
    } else {
        // 0xe and 0xf are "always" conditions
        do_end_tb_to_addr(ctx, &addr_val, false);

        // not returning
        Err(DisasException::Branch(Some(addr), None))
    }
}

pub fn disas_uncond_b_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let addr = (ctx.curr_pc() as i64 + sextract(insn as i64, 0, 26) * 4) as usize;
    let addr_val = ctx.alloc_u64(addr as u64);

    if insn & (1 << 31) != 0 {
        // BL: branch with link
        let next_pc = ctx.alloc_u64(ctx.next_pc() as u64);
        let reg = ctx.reg(30);
        Op::push_mov(ctx, &reg, &next_pc);
    }

    do_end_tb_to_addr(ctx, &addr_val, false);
    Err(DisasException::Branch(Some(addr), None))
}

pub fn disas_comp_b_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let sf = extract(insn, 31, 1) == 1;
    let op = extract(insn, 24, 1) == 1;
    let rt = extract(insn, 0, 5);
    let addr = (ctx.curr_pc() as i64 + sextract(insn as i64, 5, 19) * 4) as usize;

    let next_pc = ctx.alloc_u64(ctx.next_pc() as u64);
    let addr_val = ctx.alloc_u64(addr as u64);

    let cmp = read_cpu_reg(ctx, rt as usize, sf);
    let label_match = ctx.alloc_label();
    let zero = ctx.alloc_u64(0);

    Op::push_brc(
        ctx,
        &label_match,
        &cmp,
        &zero,
        if op { CondOp::NE } else { CondOp::EQ },
    );
    do_end_tb_to_addr(ctx, &next_pc, true); // branch not taken

    Op::push_setlbl(ctx, &label_match);
    do_end_tb_to_addr(ctx, &addr_val, false); // branch taken

    Err(DisasException::Branch(Some(addr), Some(ctx.next_pc())))
}

pub fn disas_uncond_b_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let opc = extract(insn, 21, 4);
    let op2 = extract(insn, 16, 5);
    let op3 = extract(insn, 10, 6);
    let rn = extract(insn, 5, 5) as usize;
    let op4 = extract(insn, 0, 5);

    let dst;

    if op2 != 0x1f {
        return unallocated(ctx, insn);
    }

    match opc {
        0 | 1 | 2 => {
            // br, blr, ret
            match op3 {
                0 => {
                    // br, blr, ret
                    if op4 != 0 {
                        return unallocated(ctx, insn);
                    }
                    dst = ctx.reg(rn);
                }
                _ => return unallocated(ctx, insn), // pauth unimplemented
            }
            if opc == 1 {
                // blr: load return address
                let ret_addr = ctx.alloc_u64(ctx.next_pc() as u64);
                let x30 = ctx.reg(30);
                Op::push_mov(ctx, &x30, &ret_addr);
            }
            do_end_tb_to_addr(ctx, &dst, false);
        }
        8 | 9 => return unallocated(ctx, insn), // pauth unimplemented
        4 => return unallocated(ctx, insn),     // eret impossible in EL0
        5 => {
            // drps
            if op3 != 0 || op4 != 0 || rn != 0x1f {
                return unallocated(ctx, insn);
            }
            unreachable!("DRPS not supported");
        }
        _ => return unallocated(ctx, insn),
    }

    Err(DisasException::Branch(None, None))
}

disas_stub![test_b_imm, exc];
