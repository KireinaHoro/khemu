use super::*;
use crate::guest::arm64::facility::{do_test_jump_cc, do_end_tb_to_addr};

pub fn disas_exc_sys<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), String> {
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
) -> Result<(), String> {
    if extract(insn, 4, 1) == 1 || extract(insn, 24, 1) == 1 {
        return unallocated(ctx, insn);
    }

    let addr = (ctx.curr_pc() as i64 + sextract(insn as i64, 5, 19) * 4) as usize;
    let cond = extract(insn, 0, 4);
    let addr = ctx.alloc_u64(addr as u64);

    if cond < 0xe {
        // genuine conditional branches
        let label = ctx.alloc_label();
        do_test_jump_cc(ctx, cond, &label);
        // TODO(jsteward) we do not implement TB chaining for now
        // TODO(jsteward) simply set PC and end current TB for the runtime to figure out where to go
        let next_pc = ctx.alloc_u64(ctx.next_pc() as u64);
        do_end_tb_to_addr(ctx, &next_pc);
        Op::push_setlbl(ctx, &label);
        do_end_tb_to_addr(ctx, &addr);
    } else {
        // 0xe and 0xf are "always" conditions
        do_end_tb_to_addr(ctx, &addr);
    }

    Ok(())
}

disas_stub![
    uncond_b_imm,
    comp_b_imm,
    test_b_imm,
    system,
    exc,
    uncond_b_reg
];
