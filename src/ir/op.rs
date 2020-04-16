use super::storage::KHVal;
use macros::gen_ops;

#[rustfmt::skip]
gen_ops! {
    KHVal<u64> {
        unary: Neg, Not, Mov, Bswap, ExtU, ExtS;
        binary: Add, Sub, Mul, Div, Rem, Remu; // arithmetic
        binary: And, Or, Xor, Andc, Eqv, Nand, Nor, Orc, Clz, Ctz; // logical
        binary: Shl, Shr, Sar, Rotl, Rotr; // shifts / rotates
        custom: Call, rd, func, rs1, rs2, rs3, rs4;
        //override_maker: And, Add, Call;
    },
    KHVal<f64> {
        unary: Mov, ExtU, ExtS;
        binary: Add, Sub, Mul, Div;
    }
}
