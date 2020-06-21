// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::facility::*;
use super::*;

use log::*;
use std::convert::TryInto;

pub fn disas_ldst_pair<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    trace!("ldst_pair");
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
        _ => return Err(DisasException::Unexpected("unmatched index".to_owned())),
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
            do_ldst(ctx, is_load, is_signed, false, size, &tmp, &clean_addr);
            Op::push_add(ctx, &clean_addr, &clean_addr, &size_val);
            do_ldst(ctx, is_load, is_signed, false, size, &rt2, &clean_addr);
            Op::push_mov(ctx, &rt, &tmp);
        } else {
            do_ldst(ctx, is_load, is_signed, false, size, &rt, &clean_addr);
            Op::push_add(ctx, &clean_addr, &clean_addr, &size_val);
            do_ldst(ctx, is_load, is_signed, false, size, &rt2, &clean_addr);
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

pub fn disas_ldst_reg_imm9<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    opc: u32,
    mut size: u32,
    rt: usize,
    is_vector: bool,
) -> Result<(), DisasException> {
    trace!("ldst_reg_imm9");
    let rn = extract(insn, 5, 5) as usize;
    let idx = extract(insn, 10, 2);
    let imm9 = sextract(insn as i64, 12, 9);

    let is_unpriv = idx == 2;
    let mut is_signed = false;
    let mut is_extended = false;

    let is_store;
    let post_index;
    let writeback;

    if is_vector {
        size |= (opc & 2) << 1;
        if size > 4 || is_unpriv {
            return unallocated(ctx, insn);
        }
        is_store = (opc & 1) == 0;
        if !fp_access_check(ctx) {
            return Ok(());
        }
    } else {
        if size == 3 && opc == 2 {
            // prfm - prefetch
            if idx != 0 {
                return unallocated(ctx, insn);
            }
            return Ok(());
        }
        if opc == 3 && size > 1 {
            return unallocated(ctx, insn);
        }
        is_store = opc == 0;
        is_signed = extract(opc, 1, 1) == 1;
        is_extended = size < 3 && extract(opc, 0, 1) == 1;
    }

    match idx {
        0 | 2 => {
            post_index = false;
            writeback = false;
        }
        1 => {
            post_index = true;
            writeback = true;
        }
        3 => {
            post_index = false;
            writeback = true;
        }
        _ => unreachable!(),
    }

    if rn == 31 {
        check_sp_alignment(ctx);
    }

    let dirty_addr = read_cpu_reg_sp(ctx, rn, true);
    let imm_val = ctx.alloc_u64(imm9.abs().try_into().unwrap());
    if !post_index {
        (if imm9 >= 0 {
            Op::push_add
        } else {
            Op::push_sub
        })(ctx, &dirty_addr, &dirty_addr, &imm_val);
    }
    let clean_addr = clean_data_tbi(ctx, &dirty_addr);

    let size = 1 << size as u64; // our ld / st accepts bytes

    if is_vector {
        // TODO(jsteward) implement vector/SIMD
        return unallocated(ctx, insn);
    } else {
        let rt = ctx.reg(rt);

        do_ldst(
            ctx,
            !is_store,
            is_signed,
            is_extended,
            size,
            &rt,
            &clean_addr,
        );
    }

    if writeback {
        if post_index {
            (if imm9 >= 0 {
                Op::push_add
            } else {
                Op::push_sub
            })(ctx, &dirty_addr, &dirty_addr, &imm_val);
        }
        Op::push_mov(ctx, &ctx.reg_sp(rn), &dirty_addr);
    }

    Ok(())
}

pub fn disas_ldst_atomic<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    size: u32,
    rt: usize,
    is_vector: bool,
) -> Result<(), DisasException> {
    Err(DisasException::Unexpected(
        "ldst_atomic work in progress".to_owned(),
    ))
}

pub fn disas_ldst_reg_offset<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    opc: u32,
    size: u32,
    rt: usize,
    is_vector: bool,
) -> Result<(), DisasException> {
    Err(DisasException::Unexpected(
        "ldst_reg_offset work in progress".to_owned(),
    ))
}

pub fn disas_ldst_pac<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    size: u32,
    rt: usize,
    is_vector: bool,
) -> Result<(), DisasException> {
    Err(DisasException::Unexpected(
        "ldst_pac work in progress".to_owned(),
    ))
}

pub fn disas_ldst_reg_unsigned_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
    opc: u32,
    size: u32,
    rt: usize,
    is_vector: bool,
) -> Result<(), DisasException> {
    trace!("ldst_reg_unsigned_imm");
    let rn = extract(insn, 5, 5);
    let imm12 = extract(insn, 10, 12);

    let is_store;
    let is_signed;
    let mut is_extended = false;

    let mut size = size;

    if is_vector {
        is_signed = false;
        size |= (opc & 2) << 1;
        if size > 4 {
            return unallocated(ctx, insn);
        }
        is_store = extract(opc, 0, 1) == 0;
        if !fp_access_check(ctx) {
            return Ok(());
        }
    } else {
        if size == 3 && opc == 2 {
            // prfm - prefetch, ignore
            return Ok(());
        }
        if opc == 3 && size > 1 {
            return unallocated(ctx, insn);
        }
        is_store = opc == 0;
        is_signed = extract(opc, 1, 1) == 1;
        is_extended = size < 3 && extract(opc, 0, 1) == 1;
    }

    if rn == 31 {
        check_sp_alignment(ctx);
    }
    trace!("Reading SP");
    let dirty_addr = read_cpu_reg_sp(ctx, rn as usize, true);
    let offset = ctx.alloc_u64((imm12 << size) as u64);
    trace!("Calculating offset");
    Op::push_add(ctx, &dirty_addr, &dirty_addr, &offset);
    trace!("Cleaning TBI");
    let clean_addr = clean_data_tbi(ctx, &dirty_addr);

    let size = 1 << size as u64;

    if is_vector {
        // TODO(jsteward) implement vector/SIMD
        return unallocated(ctx, insn);
    } else {
        let rt = ctx.reg(rt as usize);
        // FIXME we skipped the ISS (Instruction-Specific Syndrome) calculation
        trace!("Performing ldst");
        do_ldst(
            ctx,
            !is_store,
            is_signed,
            is_extended,
            size,
            &rt,
            &clean_addr,
        );
    }

    Ok(())
}

pub fn disas_ldst_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    trace!("ldst_reg");
    let rt = extract(insn, 0, 5) as usize;
    let opc = extract(insn, 22, 2);
    let is_vector = extract(insn, 26, 1) == 1;
    let size = extract(insn, 30, 2);

    match extract(insn, 24, 2) {
        0 => {
            if extract(insn, 21, 1) == 0 {
                disas_ldst_reg_imm9(ctx, insn, opc, size, rt, is_vector)
            } else {
                match extract(insn, 10, 2) {
                    0 => disas_ldst_atomic(ctx, insn, size, rt, is_vector),
                    2 => disas_ldst_reg_offset(ctx, insn, opc, size, rt, is_vector),
                    _ => disas_ldst_pac(ctx, insn, size, rt, is_vector),
                }
            }
        }
        1 => disas_ldst_reg_unsigned_imm(ctx, insn, opc, size, rt, is_vector),
        _ => unallocated(ctx, insn),
    }
}

disas_stub![
    ldst_excl,
    ld_lit,
    ldst_multiple_struct,
    ldst_single_struct,
    ldst_ldapr_stlr
];
