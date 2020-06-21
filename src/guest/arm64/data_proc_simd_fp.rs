// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;

pub fn disas_data_proc_simd_fp<R: HostStorage>(
    ctx: &mut Arm64GuestContext<R>,
    insn: InsnType,
) -> Result<(), DisasException> {
    (if extract(insn, 28, 1) == 1 && extract(insn, 30, 1) == 0 {
        disas_data_proc_fp
    } else {
        disas_data_proc_simd
    })(ctx, insn)
}

disas_stub![data_proc_fp, data_proc_simd];
