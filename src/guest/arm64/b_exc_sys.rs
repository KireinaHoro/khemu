use super::*;
use crate::guest::arm64::facility::*;

pub fn disas_exc_sys<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    (if extract(insn, 24, 1) == 1 {
        if extract(insn, 22, 2) == 0 {
            disas_system
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

disas_stub![test_b_imm, system, exc, uncond_b_reg];
