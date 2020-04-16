use super::*;

pub fn read_cpu_reg_sp(ctx: &mut Arm64GuestContext, reg: usize, sf: bool) -> KHVal<RegType> {
    let v = KHVal::new();
    (if sf { Op::make_mov } else { Op::make_extu })(&v, &ctx.xreg[reg]);
    v
}
