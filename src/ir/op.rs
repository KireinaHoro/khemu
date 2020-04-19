use crate::guest::GuestContext;
use crate::ir::storage::{HostStorage, KHVal, MemOp, ValueType};
use macros::gen_ops;
use std::rc::Rc;

#[rustfmt::skip]
// all types except for convert will be enforced to take arguments of the declared type
// type mnemonic (q, w, d, ..) can be omitted if there is no ambiguity
gen_ops! {
    ValueType::U64 {  // q - 64bit word
        unary: Neg, Not, Mov, Bswap;
        convert: ExtUwq, ExtSwq;  // convert 32bit to 64bit
        binary: Add, Sub, Mul, Div, Rem, Remu; // arithmetic
        binary: And, Or, Xor, Andc, Eqv, Nand, Nor, Orc, Clz, Ctz; // logical
        binary: Shl, Shr, Sar, Rotl, Rotr; // shifts / rotates
        binary: Load, Store;   // rd: reg, rs1: mem addr, rs2: `storage::MemOp`
        custom: ExtrU, rd, rs, ofs, len;  // unsigned extract
        custom: ExtrS, rd, rs, ofs, len;  // signed extract
        custom: Setc, rd, c1, c2, cc;  // set rd if c1 `cc` c2
        custom: Movc, rd, rs1, rs2, c1, c2, cc;  // rd = if c1 `cc` c2 then rs1 else rs2
        custom: Add2, rl, rh, al, ah, bl, bh; // [rh:rl] = [ah:al] + [bh:bl]
        custom: Call, rd, func, rs1, rs2, rs3, rs4;
        override_maker: Load, Store, Setc, Movc;
    },
    ValueType::U32 {  // w - 32bit word
        unary: Movw;
        convert: Extrl, Extrh;    // convert 64bit to 32bit
        binary: Orw, Xorw, Andcw;
        custom: Add2w, rl, rh, al, ah, bl, bh; // [rh:rl] = [ah:al] + [bh:bl]
    },
    ValueType::F64 {  // d - double float
        unary: MovD;
        binary: AddD, SubD, MulD, DivD;
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

impl<R: HostStorage> Op<R> {
    pub fn push_load<C: GuestContext<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        addr: &Rc<KHVal<R>>,
        mem_op: MemOp,
    ) {
        let mem_op = ctx.alloc_u64(mem_op.bits());
        ctx.push_op(Op::Load {
            rd: Rc::clone(rd),
            rs1: Rc::clone(addr),
            rs2: mem_op,
        });
    }

    pub fn push_store<C: GuestContext<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        addr: &Rc<KHVal<R>>,
        mem_op: MemOp,
    ) {
        let mem_op = ctx.alloc_u64(mem_op.bits());
        ctx.push_op(Op::Store {
            rd: Rc::clone(rd),
            rs1: Rc::clone(addr),
            rs2: mem_op,
        });
    }

    pub fn push_setc<C: GuestContext<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        c1: &Rc<KHVal<R>>,
        c2: &Rc<KHVal<R>>,
        cc: CondOp,
    ) {
        let cc = ctx.alloc_u64(cc.bits());
        ctx.push_op(Op::Setc {
            rd: Rc::clone(rd),
            c1: Rc::clone(c1),
            c2: Rc::clone(c2),
            cc,
        });
    }

    pub fn push_movc<C: GuestContext<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        rs1: &Rc<KHVal<R>>,
        rs2: &Rc<KHVal<R>>,
        c1: &Rc<KHVal<R>>,
        c2: &Rc<KHVal<R>>,
        cc: CondOp,
    ) {
        let cc = ctx.alloc_u64(cc.bits());
        ctx.push_op(Op::Movc {
            rd: Rc::clone(rd),
            rs1: Rc::clone(rs1),
            rs2: Rc::clone(rs2),
            c1: Rc::clone(c1),
            c2: Rc::clone(c2),
            cc,
        });
    }

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
