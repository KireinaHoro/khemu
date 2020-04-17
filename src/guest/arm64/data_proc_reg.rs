use super::*;

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

disas_stub![
    add_sub_ext_reg,
    add_sub_reg,
    logic_reg,
    adc_sbc,
    rotate_right_into_flags,
    evaluate_into_flags,
    cc,
    cond_select,
    data_proc_1src,
    data_proc_2src,
    data_proc_3src
];
