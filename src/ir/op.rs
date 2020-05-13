use crate::guest::DisasContext;
use crate::ir::storage::{HostStorage, KHVal, MemOp, ValueType};
use macros::gen_ops;
use std::rc::Rc;

#[rustfmt::skip]
// all types except for convert will be enforced to take arguments of the declared type
// type mnemonic (q, w, d, ..) can be omitted if there is no ambiguity
gen_ops! {
    ValueType::Label {  // control flow related special
        custom: Setlbl, label;  // set label to current location in IR
        custom: Brc, dest, c1, c2, cc; // branch to label dest if c1 `cc` c2
        override_maker: Brc;  // to accept CondOp and to allow multiple types
    },
    ValueType::U64 {  // q - 64bit word
        unary: Neg, Not, Mov, Bswap;
        convert: ExtUlq, ExtSlq;  // convert 32bit to 64bit
        convert: ExtUwq, ExtSwq;  // convert 16bit to 64bit
        convert: ExtUbq, ExtSbq;  // convert 8bit to 64bit
        binary: Add, Sub, Mul, Div, Rem, Remu; // arithmetic
        binary: And, Or, Xor, Andc, Eqv, Nand, Nor, Orc, Clz, Ctz; // logical
        binary: Shl, Shr, Sar, Rotl, Rotr; // shifts / rotates
        binary: Load, Store;   // rd: reg, rs1: mem addr, rs2: `storage::MemOp`
        // trap to runtime for TB lookup / fault injection
        custom: Trap, cause, val;
        custom: ExtrU, rd, rs, ofs, len;  // unsigned extract
        custom: ExtrS, rd, rs, ofs, len;  // signed extract
        custom: Depos, rd, rs1, rs2, ofs, len;  // deposit ofs,len of rs2 into rs1
        custom: Setc, rd, c1, c2, cc;  // set rd if c1 `cc` c2
        custom: Movc, rd, rs1, rs2, c1, c2, cc;  // rd = if c1 `cc` c2 then rs1 else rs2
        custom: Add2, rl, rh, al, ah, bl, bh; // [rh:rl] = [ah:al] + [bh:bl]
        custom: Call, rd, func, rs1, rs2, rs3, rs4;
        override_maker: Mov;
        override_maker: Load, Store; // to accept MemOp
        override_maker: Setc, Movc;  // to accept CondOp and to allow multiple types
        override_maker: Add, Sub, ExtUlq;    // simple optimizations
        override_maker: Trap;  // argument form, inject TB end
        override_maker: ExtrU, ExtrS, Depos; // to accept immediate value for ofs len
    },
    ValueType::U32 {  // w - 32bit word
        unary: Negw, Movw;
        convert: Extrl, Extrh;    // convert 64bit to 32bit
        binary: Subw;  // arithmetic
        binary: Andw, Orw, Xorw, Andcw;  // logical
        binary: Rotrw; // shifts / rotates
        binary: Sarw; // shifts
        custom: Add2w, rl, rh, al, ah, bl, bh; // [rh:rl] = [ah:al] + [bh:bl]
        override_maker: Movw;
    },
    ValueType::F64 {  // d - double float
        unary: Movd;
        binary: Addd, Subd, Muld, Divd;
        override_maker: Movd;
    }
}

// the bitfield is designed to support inverting condition or allowing equality
// with only a single bit toggle.
bitflags! {
    pub struct CondOp: u64 {
        // sign-irrelevant
        const NEVER     = 0b0000;
        const ALWAYS    = 0b0001;
        const EQ        = 0b1000;
        const NE        = 0b1001;
        // signed
        const LT        = 0b0010;
        const GE        = 0b0011;
        const LE        = 0b1010;
        const GT        = 0b1011;
        // unsigned
        const LTU       = 0b0100;
        const GEU       = 0b0101;
        const LEU       = 0b1100;
        const GTU       = 0b1101;
    }
}

impl CondOp {
    pub fn invert(&mut self) {
        self.bits = self.bits ^ 1;
    }
}

bitflags! {
    pub struct TrapOp: u64 {
        const LOOKUP_TB = 0;
        const UNDEF_OPCODE = 1;
        const ACCESS_FAULT = 2;
    }
}

// optimizations when creating Op
mod opt;
// fused Ops or those with a different interface
mod meta;
