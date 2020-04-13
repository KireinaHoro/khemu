pub mod guest;
pub mod host;
pub mod ir;
pub mod util;

pub struct EmuContext<GT, HT>
where
    GT: guest::GuestContext,
    HT: host::HostContext,
{
    pub guest: GT,
    pub host: HT,
}
