// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;
use crate::guest::arm64::facility::*;

pub fn disas_add_sub_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
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
        Op::push_extulq
    })(ctx, &rd, &result);

    Ok(())
}

pub fn disas_movw_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let rd = extract(insn, 0, 5) as usize;
    let sf = extract(insn, 31, 1) == 1;
    let opc = extract(insn, 29, 2);
    let pos = extract(insn, 21, 2) << 4;
    let mut imm = extract(insn, 5, 16) as u64;

    let rd = ctx.reg(rd);

    if !sf && pos >= 32 {
        return unallocated(ctx, insn);
    }

    match opc {
        0 | 2 => {
            // movn / movz
            imm <<= pos as u64;
            if opc == 0 {
                imm = !imm;
            }
            if !sf {
                imm &= 0xffffffffu64;
            }
            let imm = ctx.alloc_u64(imm);
            Op::push_mov(ctx, &rd, &imm);
        }
        3 => {
            // movk
            let imm = ctx.alloc_u64(imm);
            Op::push_depos(ctx, &rd, &rd, &imm, pos as u64, 16);
            if !sf {
                Op::push_extulq(ctx, &rd, &rd);
            }
        }
        _ => return unallocated(ctx, insn),
    }

    Ok(())
}

pub fn disas_pc_rel_addr<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let page = extract(insn, 31, 1);
    let rd = extract(insn, 0, 5);
    let rd = ctx.reg(rd as usize);

    let offset = sextract(insn as i64, 5, 19); // immhi
    let mut offset = offset << 2 | extract(insn, 29, 2) as i64; // immlo
    let mut base = ctx.curr_pc();

    if page == 1 {
        // ADRP: page based
        base &= !0xfff;
        offset <<= 12;
    }

    let val = ctx.alloc_u64(base as u64 + offset as u64);
    Op::push_mov(ctx, &rd, &val);

    Ok(())
}

pub fn disas_logic_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    let sf = extract(insn, 31, 1) == 1;
    let opc = extract(insn, 29, 2);
    let is_n = extract(insn, 22, 1) == 1;
    let immr = extract(insn, 16, 6);
    let imms = extract(insn, 10, 6);
    let rn = extract(insn, 5, 5) as usize;
    let rd = extract(insn, 0, 5) as usize;

    if !sf && is_n {
        return unallocated(ctx, insn);
    }

    let rd_val;

    if opc == 3 {
        rd_val = ctx.reg(rd);
    } else {
        rd_val = ctx.reg_sp(rd);
    }
    let rn_val = ctx.reg(rn);

    match logic_imm_decode_wmask(is_n as u32, imms, immr) {
        None => return unallocated(ctx, insn),
        Some(mut wmask) => {
            if !sf {
                wmask &= 0xffffffff;
            }

            let wmask = ctx.alloc_u64(wmask);
            let mut is_and = false;
            (match opc {
                3 | 0 => {
                    // ands, and
                    is_and = true;
                    Op::push_add
                }
                1 => Op::push_or,  // orr
                2 => Op::push_xor, // eor
                _ => unreachable!(),
            })(ctx, &rd_val, &rn_val, &wmask);

            if !sf && !is_and {
                Op::push_extulq(ctx, &rd_val, &rd_val);
            }

            if opc == 3 {
                // ands
                do_logic_cc(ctx, sf, &rd_val);
            }

            Ok(())
        }
    }
}

disas_stub![bitfield, extract];
