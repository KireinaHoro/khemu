use super::facility::*;
use super::*;

pub fn disas_ldst_pair<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), String> {
    let rt = extract(insn, 0, 5) as usize;
    let rn = extract(insn, 5, 5) as usize;
    let rt2 = extract(insn, 10, 5) as usize;
    let index = extract(insn, 23, 2);
    let is_vector = extract(insn, 26, 1) == 1;
    let is_load = extract(insn, 22, 1) == 1;
    let opc = extract(insn, 30, 2);
    let size: u32;

    let mut is_signed = false;
    let mut postindex = false;
    let mut wback = false;
    let mut offset = sextract(insn as u64, 15, 7);

    if opc == 3 {
        return unallocated(ctx, insn);
    }

    if is_vector {
        size = 2 + opc;
    } else {
        size = 2 + extract(opc, 1, 1);
        is_signed = extract(opc, 0, 1) == 1;
        if !is_load && is_signed {
            return unallocated(ctx, insn);
        }
    }
    offset <<= size as u64;

    match index {
        0 => {
            if is_signed {
                return unallocated(ctx, insn);
            }
            postindex = false;
        }
        1 => {
            postindex = true;
            wback = true;
        }
        2 => {
            postindex = false;
        }
        3 => {
            postindex = false;
            wback = true;
        }
        _ => return Err("unmatched index".to_owned()),
    }

    if rn == 31 {
        // sp
        check_sp_alignment(ctx);
    }

    let dirty_addr = read_cpu_reg_sp(ctx, rn, true);
    let offset = ctx.alloc_u64(offset);
    if !postindex {
        ctx.push_op(Op::make_add(&dirty_addr, &dirty_addr, &offset));
    }
    let clean_addr = clean_data_tbi(ctx, &dirty_addr);

    if is_vector {
        // TODO(jsteward) support for vector / fp
        return unallocated(ctx, insn);
    } else {
        let rt = ctx.reg(rt);
        let rt2 = ctx.reg(rt2);

        if is_load {
            // do not modify rt before recognizing any exception from the second load
            let tmp = ctx.alloc_val(ValueType::U64);
            do_ldst(
                ctx,
                is_load,
                &tmp,
                &clean_addr,
                MemOp::from_size(size) | MemOp::from_sign(is_signed),
            );
            ctx.push_op(Op::make_add(&clean_addr, &clean_addr, &size_val));
            do_ldst(
                ctx,
                is_load,
                &rt2,
                &clean_addr,
                MemOp::from_size(size) | MemOp::from_sign(is_signed),
            );
            ctx.push_op(Op::make_mov(&rt, &tmp));
        } else {
            do_ldst(
                ctx,
                is_load,
                &rt,
                &clean_addr,
                MemOp::from_size(size) | MemOp::from_sign(is_signed),
            );
            ctx.push_op(Op::make_add(&clean_addr, &clean_addr, &size_val));
            do_ldst(
                ctx,
                is_load,
                &rt2,
                &clean_addr,
                MemOp::from_size(size) | MemOp::from_sign(is_signed),
            );
        }
    }

    if wback {
        if postindex {
            ctx.push_op((if offset >= 0 {
                Op::make_add
            } else {
                Op::make_sub
            })(&dirty_addr, &dirty_addr, &offset_val));
        }
        ctx.push_op(Op::make_mov(&ctx.reg_sp(rn), &dirty_addr));
    }

    Ok(())
}

#[allow(unused)]
fn check_sp_alignment<R: HostStorage>(ctx: &mut Arm64GuestContext<R>) {
    /* sp alignment check as specified in AArch64 omitted */
}

disas_stub![
    ldst_excl,
    ld_lit,
    ldst_reg,
    ldst_multiple_struct,
    ldst_single_struct,
    ldst_ldapr_stlr
];
