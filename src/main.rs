// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use khemu::*;

use crate::runtime::*;
use khemu::host::llvm::LLVMHostContext;

fn main() -> Result<(), String> {
    env_logger::init();

    do_work::<LLVMHostContext>()
}
