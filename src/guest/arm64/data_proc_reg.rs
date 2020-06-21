// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;
use crate::guest::arm64::facility::*;

pub fn disas_data_proc_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let op0 = extract(insn, 30, 1);
    let op1 = extract(insn, 28, 1);
    let op2 = extract(insn, 21, 4);
    let op3 = extract(insn, 10, 6);

    if op1 == 0 {
        return (if op2 & 8 != 0 {
            if op2 & 1 != 0 {
                disas_add_sub_ext_reg
            } else {
                disas_add_sub_reg
            }
        } else {
            disas_logic_reg
        })(ctx, insn);
    }
    (match op2 {
        0x0 => match op3 {
            0x0 => disas_adc_sbc,
            0x1 | 0x21 => disas_rotate_right_into_flags,
            0x2 | 0x12 | 0x22 | 0x32 => disas_evaluate_into_flags,
            _ => super::unallocated,
        },
        0x2 => disas_cc,
        0x4 => disas_cond_select,
        0x6 => {
            if op0 != 0 {
                disas_data_proc_1src
            } else {
                disas_data_proc_2src
            }
        }
        0x8..=0xf => disas_data_proc_3src,
        _ => super::unallocated,
    })(ctx, insn)
}

pub fn disas_logic_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let sf = extract(insn, 31, 1) == 1;
    let opc = extract(insn, 29, 2);
    let shift_type = extract(insn, 22, 2);
    let invert = extract(insn, 21, 1);
    let rm = extract(insn, 16, 5) as usize;
    let shift_amount = extract(insn, 10, 6);
    let rn = extract(insn, 5, 5) as usize;
    let rd = extract(insn, 0, 5) as usize;

    if !sf && shift_amount & (1 << 5) != 0 {
        return unallocated(ctx, insn);
    }

    let rd = ctx.reg(rd);

    if opc == 1 && shift_amount == 0 && shift_type == 0 && rn == 31 {
        // unshifted ORR and ORN with WZR/XZR is the standard encoding for register-register MOV and MVN
        let rm = ctx.reg(rm);
        if invert == 1 {
            Op::push_not(ctx, &rd, &rm);
            if !sf {
                Op::push_extulq(ctx, &rd, &rd);
            }
        } else {
            if sf {
                Op::push_mov(ctx, &rd, &rm);
            } else {
                Op::push_extulq(ctx, &rd, &rm);
            }
        }
        return Ok(());
    }

    let rm = read_cpu_reg(ctx, rm, sf);

    let shift_type = match A64Shift::from_bits(shift_type) {
        Some(s) => s,
        None => {
            return Err(DisasException::Unexpected(
                "unknown shift_type in logic_reg".to_owned(),
            ))
        }
    };
    if shift_amount != 0 {
        do_shift_imm(ctx, &rm, &rm, sf, shift_type, shift_amount);
    }

    let rn = ctx.reg(rn);

    match opc | (invert << 2) {
        0 | 3 => {
            // and / ands
            Op::push_and(ctx, &rd, &rn, &rm);
        }
        1 => {
            // orr
            Op::push_or(ctx, &rd, &rn, &rm);
        }
        2 => {
            // eor
            Op::push_xor(ctx, &rd, &rn, &rm);
        }
        4 | 7 => {
            // bic / bics
            Op::push_andc(ctx, &rd, &rn, &rm);
        }
        5 => {
            // orn
            Op::push_orc(ctx, &rd, &rn, &rm);
        }
        6 => {
            // eon
            Op::push_eqv(ctx, &rd, &rn, &rm);
        }
        _ => {
            return Err(DisasException::Unexpected(
                "unknown opc in logic_reg".to_owned(),
            ))
        }
    }

    if !sf {
        Op::push_extulq(ctx, &rd, &rd);
    }

    if opc == 3 {
        do_logic_cc(ctx, sf, &rd);
    }

    Ok(())
}

pub fn disas_add_sub_ext_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let rd = extract(insn, 0, 5) as usize;
    let rn = extract(insn, 5, 5) as usize;
    let imm3 = extract(insn, 10, 3);
    let option = extract(insn, 13, 3);
    let rm = extract(insn, 16, 5) as usize;
    let opt = extract(insn, 22, 2);
    let setflags = extract(insn, 29, 1) == 1;
    let sub_op = extract(insn, 30, 1) == 1;
    let sf = extract(insn, 31, 1) == 1;

    if imm3 > 4 || opt != 0 {
        return unallocated(ctx, insn);
    }

    // non-flag version may use SP
    let rd = if setflags {
        ctx.reg(rd)
    } else {
        ctx.reg_sp(rd)
    };
    let rn = read_cpu_reg_sp(ctx, rn, sf);
    let rm = read_cpu_reg(ctx, rm, sf);
    do_ext_and_shift_reg(ctx, &rm, &rm, option, imm3);

    let result = ctx.alloc_val(ValueType::U64);

    if !setflags {
        (if sub_op { Op::push_sub } else { Op::push_add })(ctx, &result, &rn, &rm);
    } else {
        // update condition codes
        if sub_op {
            do_sub_cc(ctx, sf, &result, &rn, &rm);
        } else {
            do_add_cc(ctx, sf, &result, &rn, &rm);
        }
    }

    (if sf { Op::push_mov } else { Op::push_extulq })(ctx, &rd, &result);

    Ok(())
}

pub fn disas_cond_select<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    if extract(insn, 29, 1) == 1 || extract(insn, 11, 1) == 1 {
        return unallocated(ctx, insn);
    }

    let sf = extract(insn, 31, 1) == 1;
    let else_inv = extract(insn, 30, 1) == 1;
    let rm = extract(insn, 16, 5) as usize;
    let cond = extract(insn, 12, 4);
    let else_inc = extract(insn, 10, 1) == 1;
    let rn = extract(insn, 5, 5) as usize;
    let rd = extract(insn, 0, 5) as usize;

    let rd = ctx.reg(rd);
    let Arm64CC {
        mut cond,
        value: val32,
    } = test_cc(ctx, cond);
    let value = ctx.alloc_val(ValueType::U64);
    Op::push_extslq(ctx, &value, &val32);

    let one = ctx.alloc_u64(1);

    let zero = ctx.alloc_u64(0);

    if rn == 31 && rm == 31 && (else_inc ^ else_inv) {
        // cset & csetm
        cond.invert();

        Op::push_setc(ctx, &rd, &value, &zero, cond);

        if else_inv {
            Op::push_neg(ctx, &rd, &rd);
        }
    } else {
        let t_true = ctx.reg(rn);
        let t_false = read_cpu_reg(ctx, rm, true);
        if else_inc && else_inc {
            Op::push_neg(ctx, &t_false, &t_false);
        } else if else_inv {
            Op::push_not(ctx, &t_false, &t_false);
        } else if else_inc {
            Op::push_add(ctx, &t_false, &t_false, &one);
        }

        Op::push_movc(ctx, &rd, &value, &zero, &t_true, &t_false, cond);
    }

    if !sf {
        Op::push_extulq(ctx, &rd, &rd);
    }

    Ok(())
}

disas_stub![
    add_sub_reg,
    adc_sbc,
    rotate_right_into_flags,
    evaluate_into_flags,
    cc,
    data_proc_1src,
    data_proc_2src,
    data_proc_3src
];
