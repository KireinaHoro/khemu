use super::*;

pub fn disas_data_proc_simd_fp<HT>(
    ctx: &mut EmuContext<Arm64GuestContext, HT>,
    insn: InsnType,
) -> Result<(), String>
where
    HT: host::HostContext<RegType = RegType>,
{
    (if extract(insn, 28, 1) == 1 && extract(insn, 30, 1) == 0 {
        disas_data_proc_fp
    } else {
        disas_data_proc_simd
    })(ctx, insn)
}

disas_stub![data_proc_fp, data_proc_simd];
