use crate::guest::GuestContext;
use crate::ir::storage::{HostStorage, KHVal, ValueType};
use macros::gen_ops;
use std::rc::Rc;

#[rustfmt::skip]
// all types except for convert will be enforced to take arguments of the declared type
// type mnemonic (q, w, d, ..) can be omitted if there is no ambiguity
gen_ops! {
    ValueType::U64 {  // q - 64bit word
        unary: Neg, Not, Mov, Bswap;
        convert: ExtU32, ExtS32;  // convert 32bit to 64bit
        binary: Add, Sub, Mul, Div, Rem, Remu; // arithmetic
        binary: And, Or, Xor, Andc, Eqv, Nand, Nor, Orc, Clz, Ctz; // logical
        binary: Shl, Shr, Sar, Rotl, Rotr; // shifts / rotates
        binary: Load, Store;   // rd: reg, rs1: mem addr, rs2: `storage::MemOp`
        custom: ExtrU, rd, rs, ofs, len;  // unsigned extract
        custom: ExtrS, rd, rs, ofs, len;  // signed extract
        custom: Add2, rl, rh, al, ah, bl, bh; // [rh:rl] = [ah:al] + [bh:bl]
        custom: Call, rd, func, rs1, rs2, rs3, rs4;
        //override_maker: Load, Store;
    },
    ValueType::U32 {  // w - 32bit word
        unary: Movw;
        convert: Extrl, Extrh;    // convert 64bit to 32bit
        binary: Orw;
        custom: Add2w, rl, rh, al, ah, bl, bh; // [rh:rl] = [ah:al] + [bh:bl]
    },
    ValueType::F64 {  // d - double float
        unary: MovD;
        binary: AddD, SubD, MulD, DivD;
    }
}

impl<R: HostStorage> Op<R> {
    pub fn push_extr<C: GuestContext<R>>(
        ctx: &mut C,
        lo: &Rc<KHVal<R>>,
        hi: &Rc<KHVal<R>>,
        arg: &Rc<KHVal<R>>,
    ) {
        Op::push_extrl(ctx, lo, arg);
        Op::push_extrh(ctx, hi, arg);
    }
}
