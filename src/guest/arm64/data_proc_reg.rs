use super::*;
use crate::guest::arm64::facility::*;

pub fn disas_data_proc_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), String> {
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
) -> Result<(), String> {
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
                Op::push_extuwq(ctx, &rd, &rd);
            }
        } else {
            if sf {
                Op::push_mov(ctx, &rd, &rm);
            } else {
                Op::push_extuwq(ctx, &rd, &rm);
            }
        }
        return Ok(());
    }

    let rm = read_cpu_reg(ctx, rm, sf);

    let shift_type = match A64Shift::from_bits(shift_type) {
        Some(s) => s,
        None => return Err("unknown shift_type in logic_reg".to_owned()),
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
        _ => return Err("unknown opc in logic_reg".to_owned()),
    }

    if !sf {
        Op::push_extuwq(ctx, &rd, &rd);
    }

    if opc == 3 {
        do_logic_cc(ctx, sf, &rd);
    }

    Ok(())
}

disas_stub![
    add_sub_ext_reg,
    add_sub_reg,
    adc_sbc,
    rotate_right_into_flags,
    evaluate_into_flags,
    cc,
    cond_select,
    data_proc_1src,
    data_proc_2src,
    data_proc_3src
];
