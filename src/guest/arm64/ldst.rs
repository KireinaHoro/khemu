use super::*;
use facility::*;

pub fn disas_ldst_pair(ctx: &mut Arm64GuestContext, insn: InsnType) -> Result<(), String> {
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
        return unallocated(ctx, insn)
    }

    if is_vector {
        size = 2 + opc;
    } else {
        size = 2 + extract(opc, 1, 1);
        is_signed = extract(opc, 0, 1) == 1;
        if !is_load && is_signed {
            return unallocated(ctx, insn)
        }
    }  
    offset <<= size;

    match index {
        0 => {
            if is_signed {
                return unallocated(ctx, insn)
            }
            postindex = false;
        },
        1 => { postindex = true; wback = true; },
        2 => { postindex = false; },
        3 => { postindex = false; wback = true; },
        _ => return Err("unmatched index".to_owned())
    }

    if rn == 31 { // sp
        check_sp_alignment(ctx);
    }

    let dirty_addr = read_cpu_reg_sp(ctx, rn, true);

    Err("ldst_pair work in progress".to_owned())
}

fn check_sp_alignment(_ctx: &mut Arm64GuestContext) {
    /* sp alignment check as specified in AArch64 omitted */
}

disas_stub![ldst_excl, ld_lit, ldst_reg, ldst_multiple_struct, ldst_single_struct, ldst_ldapr_stlr];