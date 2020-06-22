// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

#![feature(vec_leak)]
#![feature(external_doc)]
#![allow(dead_code, unused)]
#![doc(include = "../README.md")]

#[macro_use]
extern crate bitflags;

/// The frontend.  Disassembles guest code, producing [translation blocks](struct.TranslationBlock.html) (of IR operators).
pub mod guest;

/// The backend.  Accepts [translation blocks](../guest/struct.TranslationBlock.html) and emits [host blocks](trait.HostBlock.html).
pub mod host;

/// IR definition and manipulation.
pub mod ir;

/// The runtime environment.
pub mod runtime;

/// Various utilities.
pub mod util;
