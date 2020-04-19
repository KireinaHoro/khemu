use super::*;
use crate::guest::arm64::facility::*;

pub fn disas_add_sub_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), String> {
    let rd = extract(insn, 0, 5) as usize;
    let rn = extract(insn, 5, 5) as usize;
    let shift = extract(insn, 22, 2);
    let setflags = extract(insn, 29, 1) == 1;
    let sub_op = extract(insn, 30, 1) == 1;
    let is_64bit = extract(insn, 31, 1) == 1;
    let mut imm = extract(insn, 10, 12) as u64;

    let rn = ctx.reg_sp(rn);
    let rd = if setflags {
        ctx.reg(rd)
    } else {
        ctx.reg_sp(rd)
    };
    let result = ctx.alloc_val(ValueType::U64);

    if shift == 0x1 {
        imm <<= 12;
    } else if shift != 0x0 {
        return unallocated(ctx, insn);
    }

    let imm = ctx.alloc_u64(imm);
    if !setflags {
        (if sub_op { Op::push_sub } else { Op::push_add })(ctx, &result, &rn, &imm);
    } else {
        // update condition codes
        if sub_op {
            do_sub_cc(ctx, is_64bit, &result, &rn, &imm);
        } else {
            do_add_cc(ctx, is_64bit, &result, &rn, &imm);
        }
    }

    (if is_64bit {
        Op::push_mov
    } else {
        Op::push_extuwq
    })(ctx, &rd, &result);

    Ok(())
}

disas_stub![pc_rel_addr, logic_imm, movw_imm, bitfield, extract];
