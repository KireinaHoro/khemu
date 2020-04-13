extern crate num_traits;

pub mod arm64;

use crate::*;
use num_traits::Num;

pub trait GuestContext
where
    Self: Sized,
{
    type RegType: Num;
    type InsnType;
    fn next_insn(&mut self) -> Option<Self::InsnType>;
    fn disas_loop<HT>(ctx: &mut EmuContext<Self, HT>) -> Result<(), String>
    where
        HT: host::HostContext<RegType = Self::RegType>;
}
