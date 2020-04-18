use super::storage::ValueType;
use macros::gen_ops;

#[rustfmt::skip]
gen_ops! {
    ValueType::U64 {
        unary: Neg, Not, Mov, Bswap, ExtU, ExtS;
        binary: Add, Sub, Mul, Div, Rem, Remu; // arithmetic
        binary: And, Or, Xor, Andc, Eqv, Nand, Nor, Orc, Clz, Ctz; // logical
        binary: Shl, Shr, Sar, Rotl, Rotr; // shifts / rotates
        binary: Load, Store;   // rd: reg, rs1: mem addr, rs2: `storage::MemOp`
        custom: ExU, rd, rs, ofs, len;  // unsigned extract
        custom: ExS, rd, rs, ofs, len;  // signed extract
        custom: Call, rd, func, rs1, rs2, rs3, rs4;
        //override_maker: Load, Store;
    },
    ValueType::F64 {
        unary: FMov;
        binary: FAdd, FSub, FMul, FDiv;
    }
}
