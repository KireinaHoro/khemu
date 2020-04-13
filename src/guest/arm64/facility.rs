use super::*;

pub fn read_cpu_reg_sp<HT>(
    ctx: &mut EmuContext<Arm64GuestContext, HT>,
    reg: usize,
    sf: bool,
) -> ir::KHVal<RegType>
where
    HT: host::HostContext<RegType = RegType>,
{
    let v = ir::KHVal::new();
    (if sf { ir::Mov::emit } else { ir::ExtU::emit })(
        ctx.host.get_emitter(),
        &v,
        &ctx.guest.xreg[reg],
    );
    v
}
