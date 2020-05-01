use super::*;
use super::{CondOp, MemOp};

// read CPU register.
// the SP encoding is used to represent XZR (hardwired zero) in some contexts.
// If SP is needed, use `read_cpu_reg_sp`.
#[allow(dead_code)]
pub fn read_cpu_reg<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    reg: usize,
    sf: bool,
) -> Rc<KHVal<R>> {
    let v = ctx.alloc_val(ValueType::U64);
    let src = ctx.reg(reg);
    (if sf { Op::push_mov } else { Op::push_extuwq })(ctx, &v, &src);
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
    (if sf { Op::push_mov } else { Op::push_extuwq })(ctx, &v, &src);
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
        let ofs = ctx.alloc_u64(0);
        let len = ctx.alloc_u64(56);
        Op::push_extrs(ctx, dst, src, &ofs, &len);
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
    reg: &Rc<KHVal<R>>,
    addr: &Rc<KHVal<R>>,
    mem_op: MemOp,
) {
    (if is_load {
        Op::push_load
    } else {
        Op::push_store
    })(ctx, reg, addr, mem_op | MemOp::GUEST_LE);
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
    Op::push_orw(ctx, &zf, &zf, &nf);
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
        Op::push_add2w(ctx, &nf, &cf, &t0_32, &zero, &t1_32, &zero);
        Op::push_movw(ctx, &zf, &nf);
        Op::push_xorw(ctx, &vf, &nf, &t0_32);
        Op::push_xorw(ctx, &tmp, &t0_32, &t1_32);
        Op::push_andcw(ctx, &vf, &vf, &tmp);
        Op::push_extuwq(ctx, dest, &nf);
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
        Op::push_subw(ctx, &nf, &t0_32, &t1_32);
        Op::push_movw(ctx, &zf, &nf);
        Op::push_setc(ctx, &cf, &t0_32, &t1_32, CondOp::GEU);
        Op::push_xorw(ctx, &vf, &nf, &t0_32);
        Op::push_xorw(ctx, &tmp, &t0_32, &t1_32);
        Op::push_andcw(ctx, &vf, &vf, &tmp);
        Op::push_extuwq(ctx, dest, &nf);
    }
}

struct Arm64CC<R: HostStorage> {
    cond: CondOp,
    value: Rc<KHVal<R>>,
}

fn test_cc<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, cc: u32) -> Arm64CC<R> {
    let mut cond: CondOp;
    let value: Rc<KHVal<R>>;
    let (nf, zf, cf, vf) = get_flags(ctx);

    match cc {
        0 | 1 => { // eq: Z; ne: !Z
            cond = CondOp::EQ;
            value = Rc::clone(&zf);
        },
        2 | 3 => { // cs: C; cc: !C
            cond = CondOp::NE;
            value = Rc::clone(&cf);
        },
        4 | 5 => { // mi: N; pl: !N
            cond = CondOp::LT;
            value = Rc::clone(&nf);
        },
        6 | 7 => { // vs: V; vc: !V
            cond = CondOp::LT;
            value = Rc::clone(&vf);
        },
        8 | 9 => { // hi: C && !Z; ls: !(C && !Z)
            cond = CondOp::NE;
            value = ctx.alloc_val(ValueType::U32);
            Op::push_negw(ctx, &value, &cf);
            Op::push_andw(ctx, &value, &value, &zf);
        },
        10 | 11 => { // ge: N ^ V == 0; lt: N ^ V != 0
            cond = CondOp::GE;
            value = ctx.alloc_val(ValueType::U32);
            Op::push_xorw(ctx, &value, &vf, &nf);
        },
        12 | 13 => { // gt: !Z && N == V; Z || N != V
            cond = CondOp::NE;
            value = ctx.alloc_val(ValueType::U32);
            let shift = ctx.alloc_u32(31);
            Op::push_xorw(ctx, &value, &vf, &nf);
            Op::push_sarw(ctx, &value, &value, &shift);
            Op::push_andcw(ctx, &value, &zf, &value);
        },
        14 | 15 => { // always
            cond = CondOp::ALWAYS;
            value = Rc::clone(&zf);
        },
        _ => unreachable!("bad condition code {:#x}", cc)
    }

    if cc & 1 == 1 && cc != 14 && cc != 15 {
        cond.invert();
    }

    Arm64CC { cond, value }
}

pub fn do_test_jump_cc<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, cc: u32, label: &Rc<KHVal<R>>) {
    assert_eq!(label.ty, ValueType::Label);
    let Arm64CC { cond, value } = test_cc(ctx, cc);
    let zero = ctx.alloc_u32(0);
    Op::push_brc(ctx, label, &value, &zero, cond);
}

// set PC and return to runtime to find out next TB
// TODO(jsteward) we should implement TB chaining here
pub fn do_end_tb_to_addr<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, dest: &Rc<KHVal<R>>) {
    let pc = Rc::clone(&ctx.pc);
    Op::push_mov(ctx, &pc, dest);
    Op::push_trap(ctx);
}
