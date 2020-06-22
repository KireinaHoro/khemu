// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;
use super::{CondOp, MemOp};

// read CPU register.
// the SP encoding is used to represent XZR (hardwired zero) in some contexts.
// If SP is needed, use `read_cpu_reg_sp`.
pub fn read_cpu_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    reg: usize,
    sf: bool,
) -> Rc<KHVal<R>> {
    let v = ctx.alloc_val(ValueType::U64);
    let src = ctx.reg(reg);
    (if sf { Op::push_mov } else { Op::push_extulq })(ctx, &v, &src);
    v
}

// read CPU register, containing SP.
pub fn read_cpu_reg_sp<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    reg: usize,
    sf: bool,
) -> Rc<KHVal<R>> {
    let v = ctx.alloc_val(ValueType::U64);
    let src = ctx.reg_sp(reg);
    (if sf { Op::push_mov } else { Op::push_extulq })(ctx, &v, &src);
    v
}

pub fn top_byte_ignore<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    dst: &Rc<KHVal<R>>,
    src: &Rc<KHVal<R>>,
    tbi: u32,
) {
    // note that we've ignored the case of EL2 and EL3 in which the address
    // does not have two ranges, thus the tag bytes will be forced to be zero.
    // We're performing system emulation, so we assume that we always run in EL0.
    if tbi == 0 {
        Op::push_mov(ctx, dst, src);
    } else {
        Op::push_extrs(ctx, dst, src, 0, 56);
    }
}

pub fn clean_data_tbi<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    addr: &Rc<KHVal<R>>,
) -> Rc<KHVal<R>> {
    let ret = ctx.alloc_val(ValueType::U64);
    // Linux aarch64 enables TBI for EL0: https://www.kernel.org/doc/Documentation/arm64/tagged-pointers.txt
    // we assume TBI always happens
    top_byte_ignore(ctx, &ret, addr, 1);
    ret
}

// generate load / store with proper memory operation
pub fn do_ldst<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    is_load: bool,
    sign: bool,
    extend: bool,
    size: u64,
    reg: &Rc<KHVal<R>>,
    addr: &Rc<KHVal<R>>,
) {
    (if is_load {
        Op::push_load
    } else {
        Op::push_store
    })(
        ctx,
        reg,
        addr,
        MemOp::from_sign(sign) | MemOp::from_size(size) | MemOp::GUEST_LE,
    );

    if is_load && extend && sign {
        assert!(size < 8);
        Op::push_extulq(ctx, reg, reg);
    }
}

fn get_flags<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
) -> (Rc<KHVal<R>>, Rc<KHVal<R>>, Rc<KHVal<R>>, Rc<KHVal<R>>) {
    (
        Rc::clone(&ctx.nf),
        Rc::clone(&ctx.zf),
        Rc::clone(&ctx.cf),
        Rc::clone(&ctx.vf),
    )
}

fn set_nz64<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, v: &Rc<KHVal<R>>) {
    let (nf, zf, _, _) = get_flags(ctx);
    assert_eq!(v.ty, ValueType::U64);
    Op::push_extr(ctx, &zf, &nf, v);
    Op::push_orl(ctx, &zf, &zf, &nf);
}

// generate add with condition code modification
pub fn do_add_cc<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    sf: bool,
    dest: &Rc<KHVal<R>>,
    t0: &Rc<KHVal<R>>,
    t1: &Rc<KHVal<R>>,
) {
    let (nf, zf, cf, vf) = get_flags(ctx);
    if sf {
        let flag = ctx.alloc_val(ValueType::U64);
        let tmp = ctx.alloc_val(ValueType::U64);

        let zero = ctx.alloc_u64(0);
        // capture carry in flag
        Op::push_add2(ctx, dest, &flag, t0, &zero, t1, &zero);
        Op::push_extrl(ctx, &cf, &flag);
        set_nz64(ctx, dest);
        // calculate vf
        Op::push_xor(ctx, &flag, dest, t0);
        Op::push_xor(ctx, &tmp, t0, t1);
        Op::push_andc(ctx, &flag, &flag, &tmp);
        Op::push_extrh(ctx, &vf, &flag);
    } else {
        let t0_32 = ctx.alloc_val(ValueType::U32);
        let t1_32 = ctx.alloc_val(ValueType::U32);
        let tmp = ctx.alloc_val(ValueType::U32);

        let zero = ctx.alloc_u32(0);
        Op::push_extrl(ctx, &t0_32, t0);
        Op::push_extrl(ctx, &t1_32, t1);
        Op::push_add2l(ctx, &nf, &cf, &t0_32, &zero, &t1_32, &zero);
        Op::push_mov(ctx, &zf, &nf);
        Op::push_xorl(ctx, &vf, &nf, &t0_32);
        Op::push_xorl(ctx, &tmp, &t0_32, &t1_32);
        Op::push_andcl(ctx, &vf, &vf, &tmp);
        Op::push_extulq(ctx, dest, &nf);
    }
}

// generate sub with condition code modification
pub fn do_sub_cc<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    sf: bool,
    dest: &Rc<KHVal<R>>,
    t0: &Rc<KHVal<R>>,
    t1: &Rc<KHVal<R>>,
) {
    let (nf, zf, cf, vf) = get_flags(ctx);
    if sf {
        let flag = ctx.alloc_val(ValueType::U64);
        let tmp = ctx.alloc_val(ValueType::U64);

        Op::push_sub(ctx, dest, t0, t1);
        set_nz64(ctx, dest);
        // calculate cf
        Op::push_setc(ctx, &flag, t0, t1, CondOp::GEU);
        Op::push_extrl(ctx, &cf, &flag);
        // calculate vf
        Op::push_xor(ctx, &flag, &dest, t0);
        Op::push_xor(ctx, &tmp, t0, t1);
        Op::push_andc(ctx, &flag, &flag, &tmp);
        Op::push_extrh(ctx, &vf, &flag);
    } else {
        let t0_32 = ctx.alloc_val(ValueType::U32);
        let t1_32 = ctx.alloc_val(ValueType::U32);
        let tmp = ctx.alloc_val(ValueType::U32);

        Op::push_extrl(ctx, &t0_32, t0);
        Op::push_extrl(ctx, &t1_32, t1);
        Op::push_subl(ctx, &nf, &t0_32, &t1_32);
        Op::push_mov(ctx, &zf, &nf);
        Op::push_setc(ctx, &cf, &t0_32, &t1_32, CondOp::GEU);
        Op::push_xorl(ctx, &vf, &nf, &t0_32);
        Op::push_xorl(ctx, &tmp, &t0_32, &t1_32);
        Op::push_andcl(ctx, &vf, &vf, &tmp);
        Op::push_extulq(ctx, dest, &nf);
    }
}

pub struct Arm64CC<R: HostStorage> {
    pub cond: CondOp,
    pub value: Rc<KHVal<R>>,
}

pub fn test_cc<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, cc: u32) -> Arm64CC<R> {
    let mut cond: CondOp;
    let value: Rc<KHVal<R>>;
    let (nf, zf, cf, vf) = get_flags(ctx);

    match cc {
        0 | 1 => {
            // eq: Z; ne: !Z
            cond = CondOp::EQ;
            value = Rc::clone(&zf);
        }
        2 | 3 => {
            // cs: C; cc: !C
            cond = CondOp::NE;
            value = Rc::clone(&cf);
        }
        4 | 5 => {
            // mi: N; pl: !N
            cond = CondOp::LT;
            value = Rc::clone(&nf);
        }
        6 | 7 => {
            // vs: V; vc: !V
            cond = CondOp::LT;
            value = Rc::clone(&vf);
        }
        8 | 9 => {
            // hi: C && !Z; ls: !(C && !Z)
            cond = CondOp::NE;
            value = ctx.alloc_val(ValueType::U32);
            Op::push_negl(ctx, &value, &cf);
            Op::push_andl(ctx, &value, &value, &zf);
        }
        10 | 11 => {
            // ge: N ^ V == 0; lt: N ^ V != 0
            cond = CondOp::GE;
            value = ctx.alloc_val(ValueType::U32);
            Op::push_xorl(ctx, &value, &vf, &nf);
        }
        12 | 13 => {
            // gt: !Z && N == V; Z || N != V
            cond = CondOp::NE;
            value = ctx.alloc_val(ValueType::U32);
            let shift = ctx.alloc_u32(31);
            Op::push_xorl(ctx, &value, &vf, &nf);
            Op::push_sarl(ctx, &value, &value, &shift);
            Op::push_andcl(ctx, &value, &zf, &value);
        }
        14 | 15 => {
            // always
            cond = CondOp::ALWAYS;
            value = Rc::clone(&zf);
        }
        _ => unreachable!("bad condition code {:#x}", cc),
    }

    if cc & 1 == 1 && cc != 14 && cc != 15 {
        cond.invert();
    }

    Arm64CC { cond, value }
}

pub fn do_test_jump_cc<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    cc: u32,
    label: &Rc<KHVal<R>>,
) {
    assert_eq!(label.ty, ValueType::Label);
    let Arm64CC { cond, value } = test_cc(ctx, cc);
    let zero = ctx.alloc_u32(0);
    Op::push_brc(ctx, label, &value, &zero, cond);
}

// set PC and return to runtime to find out next TB
pub fn do_end_tb_to_addr<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    dest: &Rc<KHVal<R>>,
    is_aux: bool,
) {
    let pc = Rc::clone(&ctx.pc);
    Op::push_mov(ctx, &pc, dest);
    Op::push_trap(ctx, TrapOp::LOOKUP_TB, dest);
    if is_aux {
        ctx.set_aux_chain();
    } else {
        ctx.set_direct_chain();
    }
}

bitflags! {
    pub struct A64Shift: u32 {
        const LSL = 0;
        const LSR = 1;
        const ASR = 2;
        const ROR = 3;
    }
}

// shift KHVal by KHVal according to ARM shifting types
// caller needs to ensure shift is not out of range and provide semantics for out of
// range shifts per ARM mandated
pub fn do_shift<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    dest: &Rc<KHVal<R>>,
    src: &Rc<KHVal<R>>,
    sf: bool,
    shift_type: A64Shift,
    shift_amount: &Rc<KHVal<R>>,
) {
    match shift_type {
        A64Shift::LSL => Op::push_shl(ctx, dest, src, shift_amount),
        A64Shift::LSR => Op::push_shr(ctx, dest, src, shift_amount),
        A64Shift::ASR => {
            if !sf {
                Op::push_extslq(ctx, dest, src);
            }
            Op::push_sar(ctx, dest, if sf { src } else { dest }, shift_amount)
        }
        A64Shift::ROR => {
            if sf {
                Op::push_rotr(ctx, dest, src, shift_amount);
            } else {
                let t0 = ctx.alloc_val(ValueType::U32);
                let t1 = ctx.alloc_val(ValueType::U32);
                Op::push_extrl(ctx, &t0, src);
                Op::push_extrl(ctx, &t1, shift_amount);
                Op::push_rotrl(ctx, &t0, &t0, &t1);
                Op::push_extulq(ctx, dest, &t0);
            }
        }
        _ => unreachable!(),
    }

    if !sf {
        Op::push_extulq(ctx, dest, dest);
    }
}

pub fn do_shift_imm<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    dest: &Rc<KHVal<R>>,
    src: &Rc<KHVal<R>>,
    sf: bool,
    shift_type: A64Shift,
    shift_i: u32,
) {
    if sf {
        assert!(shift_i < 64);
    } else {
        assert!(shift_i < 32);
    }

    if shift_i == 0 {
        Op::push_mov(ctx, dest, src);
    } else {
        let shift_i = ctx.alloc_u64(shift_i as u64);
        do_shift(ctx, dest, src, sf, shift_type, &shift_i);
    }
}

pub fn do_ext_and_shift_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    dest: &Rc<KHVal<R>>,
    src: &Rc<KHVal<R>>,
    option: u32,
    shift: u32,
) {
    let extsize = extract(option, 0, 2);
    let is_signed = extract(option, 2, 1) == 1;

    (if is_signed {
        match extsize {
            0 => Op::push_extsbq,
            1 => Op::push_extslq,
            2 => Op::push_extslq,
            3 => Op::push_mov,
            _ => unreachable!(),
        }
    } else {
        match extsize {
            0 => Op::push_extubq,
            1 => Op::push_extulq,
            2 => Op::push_extulq,
            3 => Op::push_mov,
            _ => unreachable!(),
        }
    })(ctx, dest, src);

    if shift != 0 {
        let shift = ctx.alloc_u64(shift as u64);
        Op::push_shl(ctx, dest, dest, &shift);
    }
}

pub fn do_logic_cc<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    sf: bool,
    result: &Rc<KHVal<R>>,
) {
    let (nf, zf, cf, vf) = get_flags(ctx);
    if sf {
        set_nz64(ctx, result);
    } else {
        Op::push_extrl(ctx, &zf, result);
        Op::push_mov(ctx, &nf, &zf);
    }
    let zero = ctx.alloc_u32(0);
    Op::push_mov(ctx, &cf, &zero);
    Op::push_mov(ctx, &vf, &zero);
}

fn bitfield_replicate(mut mask: u64, mut e: u32) -> u64 {
    assert_ne!(e, 0);

    while e < 64 {
        mask |= mask << e;
        e *= 2;
    }

    mask
}

pub fn logic_imm_decode_wmask(immn: u32, imms: u32, immr: u32) -> Option<u64> {
    assert!(immn < 2);
    assert!(imms < 64);
    assert!(immr < 64);

    /* The bit patterns we create here are 64 bit patterns which
     * are vectors of identical elements of size e = 2, 4, 8, 16, 32 or
     * 64 bits each. Each element contains the same value: a run
     * of between 1 and e-1 non-zero bits, rotated within the
     * element by between 0 and e-1 bits.
     *
     * The element size and run length are encoded into immn (1 bit)
     * and imms (6 bits) as follows:
     * 64 bit elements: immn = 1, imms = <length of run - 1>
     * 32 bit elements: immn = 0, imms = 0 : <length of run - 1>
     * 16 bit elements: immn = 0, imms = 10 : <length of run - 1>
     *  8 bit elements: immn = 0, imms = 110 : <length of run - 1>
     *  4 bit elements: immn = 0, imms = 1110 : <length of run - 1>
     *  2 bit elements: immn = 0, imms = 11110 : <length of run - 1>
     * Notice that immn = 0, imms = 11111x is the only combination
     * not covered by one of the above options; this is reserved.
     * Further, <length of run - 1> all-ones is a reserved pattern.
     *
     * In all cases the rotation is by immr % e (and immr is 6 bits).
     */

    let len = 31 - (immn << 6 | !imms & 0x3f).leading_zeros();
    if len < 1 {
        return None;
    }

    let e = 1 << len;
    let levels = e - 1;
    let s = imms & levels;
    let r = immr & levels;

    if s == levels {
        // <length of run - 1> mustn't be all-ones
        return None;
    }

    fn bitmask64(len: u32) -> u64 {
        assert!(len > 0 && len <= 64);
        !0u64 >> (64 - len as u64)
    }

    let mut mask = bitmask64(s + 1);
    if r != 0 {
        mask = (mask >> r as u64) | (mask << (e - r) as u64);
        mask &= bitmask64(e);
    }

    mask = bitfield_replicate(mask, e);

    Some(mask)
}

// check that FP/neon is enabled
// if not enabled, the caller should not emit any code for the instruction
pub fn fp_access_check<R: HostStorage>(ctx: &mut Arm64GuestContext<R>) -> bool {
    // FP not enabled yet, always disabled
    Op::push_trap(ctx, TrapOp::UNDEF_OPCODE, &Rc::clone(&ctx.pc));

    false
}
