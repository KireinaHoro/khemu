#![feature(vec_leak)]

#[macro_use]
extern crate bitflags;

pub mod guest;
pub mod host;
pub mod ir;
pub mod runtime;
pub mod util;
