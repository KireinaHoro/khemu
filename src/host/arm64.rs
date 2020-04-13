// AArch64 Registers
macro_rules! gen_reg {
    ($($reg:ident),*) => {
        paste::item! {
            pub enum RegInfo {
                $(
                    [< Reg $reg >]
                ),*
            }
        }
    }
}

gen_reg![
    X0, X1, X2, X3, X4, X5, X6, X7, X8, X9, X10, X11, X12, X13, X14, X15, X16, X17, X18, X19, X20,
    X21, X22, X23, X24, X25, X26, X27, X28, X29, X30, XZR, V0, V1, V2, V3, V4, V5, V6, V7, V8, V9,
    V10, V11, V12, V13, V14, V15, V16, V17, V18, V19, V20, V21, V22, V23, V24, V25, V26, V27, V28,
    V29, V30, V31
];

#[allow(non_upper_case_globals)]
impl RegInfo {
    pub const RegFP: RegInfo = RegInfo::RegX29;
    pub const RegLR: RegInfo = RegInfo::RegX30;
    pub const RegSP: RegInfo = RegInfo::RegXZR;
}
