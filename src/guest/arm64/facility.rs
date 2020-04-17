use super::*;

pub fn read_cpu_reg_sp<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    reg: usize,
    sf: bool,
) -> Rc<KHVal<R>> {
    let v = ctx.alloc_val(ValueType::U64);
    ctx.push_op((if sf { Op::make_mov } else { Op::make_extu })(
        &v,
        &ctx.xreg[reg],
    ));
    v
}
