use super::MemOp;
use super::*;

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
    ctx.push_op((if sf { Op::make_mov } else { Op::make_extu })(&v, &src));
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
    ctx.push_op((if sf { Op::make_mov } else { Op::make_extu })(&v, &src));
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
        ctx.push_op(Op::make_mov(dst, src));
    } else {
        let ofs = ctx.alloc_u64(0);
        let len = ctx.alloc_u64(56);
        ctx.push_op(Op::make_exs(dst, src, &ofs, &len));
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

pub fn do_ldst<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    is_load: bool,
    reg: &Rc<KHVal<R>>,
    addr: &Rc<KHVal<R>>,
    mem_op: MemOp,
) {
    let mem_op = ctx.alloc_u64((mem_op | MemOp::GUEST_LE).bits());
    ctx.push_op((if is_load {
        Op::make_load
    } else {
        Op::make_store
    })(reg, addr, &mem_op));
}
