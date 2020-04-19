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
    let mem_op = ctx.alloc_u64((mem_op | MemOp::GUEST_LE).bits());
    (if is_load {
        Op::push_load
    } else {
        Op::push_store
    })(ctx, reg, addr, &mem_op);
}

pub fn set_nz<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, v: &Rc<KHVal<R>>) {}

pub fn do_addsub_cc<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    is_add: bool,
    sf: bool,
    dest: &Rc<KHVal<R>>,
    t0: &Rc<KHVal<R>>,
    t1: &Rc<KHVal<R>>,
) {
    if sf {}
}
