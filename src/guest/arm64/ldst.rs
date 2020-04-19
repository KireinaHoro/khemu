use super::facility::*;
use super::*;

use std::convert::TryInto;

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
    let postindex;
    let wback;

    let mut is_signed = false;
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
    let offset = sextract(insn as i64, 15, 7) << (size as i64);

    match index {
        0 => {
            // signed offset with non-temporal hint
            if is_signed {
                // no non-temporal hint version of LDPSW
                return unallocated(ctx, insn);
            }
            postindex = false;
            wback = false;
        }
        1 => {
            // post-index
            postindex = true;
            wback = true;
        }
        2 => {
            // signed offset, no wback
            postindex = false;
            wback = false;
        }
        3 => {
            // pre-index
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
    let offset_val = ctx.alloc_u64(offset.abs().try_into().unwrap());
    let size = 1 << size as u64;
    let size_val = ctx.alloc_u64(size);
    if !postindex {
        (if offset >= 0 {
            Op::push_add
        } else {
            Op::push_sub
        })(ctx, &dirty_addr, &dirty_addr, &offset_val);
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
            Op::push_add(ctx, &clean_addr, &clean_addr, &size_val);
            do_ldst(
                ctx,
                is_load,
                &rt2,
                &clean_addr,
                MemOp::from_size(size) | MemOp::from_sign(is_signed),
            );
            Op::push_mov(ctx, &rt, &tmp);
        } else {
            do_ldst(
                ctx,
                is_load,
                &rt,
                &clean_addr,
                MemOp::from_size(size) | MemOp::from_sign(is_signed),
            );
            Op::push_add(ctx, &clean_addr, &clean_addr, &size_val);
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
            (if offset >= 0 {
                Op::push_add
            } else {
                Op::push_sub
            })(ctx, &dirty_addr, &dirty_addr, &offset_val);
        }
        Op::push_mov(ctx, &ctx.reg_sp(rn), &dirty_addr);
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
