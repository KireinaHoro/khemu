use crate::util::*;
use super::*;

pub fn disas_exc_sys<R: HostStorage>(ctx: &mut Arm64GuestContext<R>, insn: InsnType) -> Result<(), String> {
    (if extract(insn, 24, 1) == 1 {
        if extract(insn, 22, 2) == 0 {
            disas_system
        } else {
            super::unallocated
        }
    } else {
        disas_exc
    })(ctx, insn)
}

disas_stub![uncond_b_imm, comp_b_imm, test_b_imm, cond_b_imm, system, exc, uncond_b_reg];