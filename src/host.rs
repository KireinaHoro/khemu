extern crate num_traits;

use super::ir;
pub use ir::Emitter;
use num_traits::Num;

pub mod arm64;
pub mod dump_ir;

pub trait HostContext {
    type RegType: Num;
    type EM: Emitter<ValType = Self::RegType>;
    fn get_emitter(&mut self) -> &mut Self::EM;
}
