#[macro_use]
extern crate bitflags;

pub mod guest;
pub mod host;
pub mod ir;
pub mod util;

pub struct CodeGenContext<GT, HT>
where
    HT: host::HostContext,
    GT: guest::GuestContext<HT::StorageType>,
{
    pub guest: GT,
    pub host: HT,
}
