// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use crate::guest::DisasContext;
use crate::ir::storage::{HostStorage, KHVal, MemOp, ValueType};
use macros::gen_ops;
use std::fmt::{Display, Error, Formatter};
use std::rc::Rc;

#[rustfmt::skip]
// all types except for convert will be enforced to take arguments of the declared type
// type mnemonic (q, w, d, ..) can be omitted if there is no ambiguity
gen_ops! {
    ValueType::Label {
        /// Set `label` to current insertion point in translated block.
        custom: Setlbl, label;
        /// Conditional branch to label `dest`.
        ///
        /// Branches if `c1` and `c2` satisfies `cc`, or `c1 _cc_ c2`.
        custom: Brc, dest, c1, c2, cc;
        override_maker: Brc;
    },
    ValueType::U64 {
        /// Basic unary operators for `U64` IR registers.
        ///
        /// Notable ones:
        /// - Mov: duplicate register (as in SSA paradigm)
        /// - Bswap: byte swap
        unary: Neg, Not, Mov, Bswap;
        /// Extend lower 32 bit (long) to full word (quad), unsigned (zero extension) or signed
        /// (sign extension).
        convert: ExtUlq, ExtSlq;
        /// Extend lower 16 bit (word) to full word (quad), unsigned (zero extension) or signed
        /// (sign extension).
        convert: ExtUwq, ExtSwq;
        /// Extend lower 8 bit (byte) to full word (quad), unsigned (zero extension) or signed
        /// (sign extension).
        convert: ExtUbq, ExtSbq;
        /// Basic binary arithmetic operators for `U64` IR registers.
        ///
        /// Notable ones:
        /// - Rem: take remainder, signed
        /// - Remu: take remainder, unsigned
        binary: Add, Sub, Mul, Div, Rem, Remu;
        /// Basic logical (bitwise) arithmetic operators for `U64` IR registers.
        ///
        /// Notable ones:
        /// - Andc: `!(a & b)`
        /// - Orc: `!(a | b)`
        /// - Clz: count leading zeroes
        /// - Ctz: count tail zeroes
        binary: And, Or, Xor, Andc, Eqv, Nand, Nor, Orc, Clz, Ctz;
        /// Bit shift/rotations for `U64` IR registers.
        ///
        /// Notable ones:
        /// - Shr: logical right shift (zero extension)
        /// - Sar: arithmetic right shift (sign extension)
        binary: Shl, Shr, Sar, Rotl, Rotr;
        /// Load and store for `U64` IR registers.
        ///
        /// Instruction format:
        /// - `rd`: register
        /// - `rs1`: memory access target
        /// - `rs2`: memory operation mode (see [`MemOp`](../storage/struct.MemOp.html))
        binary: Load, Store;
        /// Trap to runtime.  Refer to [`TrapOP`](struct.TrapOp.html) for trap cause and value
        /// definitions.
        custom: Trap, cause, val;
        /// Signed bitfield extraction.
        ///
        /// Instruction format:
        /// - `rd`: destination register
        /// - `rs`: source register
        /// - `ofs`: offset from LSB
        /// - `len`: length of extracted bitfield
        custom: ExtrU, rd, rs, ofs, len;
        /// Unsigned bitfield extraction.
        ///
        /// Instruction format:
        /// - `rd`: destination register
        /// - `rs`: source register
        /// - `ofs`: offset from LSB
        /// - `len`: length of extracted bitfield
        custom: ExtrS, rd, rs, ofs, len;
        /// Bitfield deposit.  Overrides the destination bitfield inside `rs1` with contents from
        /// `rs2` while leaving the rest bits intact.
        ///
        /// Instruction format:
        /// - `rd`: destination register
        /// - `rs1`: source of bits outside bitfield
        /// - `rs2`: source of bits in bitfield
        /// - `ofs`: offset from LSB
        /// - `len`: length of bitfield
        custom: Depos, rd, rs1, rs2, ofs, len;
        /// Conditional set.
        ///
        /// Sets the LSB to 1 in `rd` if `c1` and `c2` satisfies `cc`, or `c1 _cc_ c2`, 0 otherwise.
        custom: Setc, rd, c1, c2, cc;
        /// Conditional move.
        ///
        /// Moves `rs1` into `rd` if `c1` and `c2` satisfies `cc`, or `c1 _cc_ c2`, `rs2` otherwise.
        custom: Movc, rd, rs1, rs2, c1, c2, cc;  // rd = if c1 `cc` c2 then rs1 else rs2
        /// Double add fused into a single instruction for `U64` IR registers.
        ///
        /// `[rh:rl] = [ah:al] + [bh:bl]`
        custom: Add2, rl, rh, al, ah, bl, bh;
        override_maker: Mov;
        override_maker: Load, Store; // to accept MemOp
        override_maker: Setc, Movc;  // to accept CondOp and to allow multiple types
        override_maker: Add, Sub, ExtUlq;    // simple optimizations
        override_maker: Trap;  // argument form, inject TB end
        override_maker: ExtrU, ExtrS, Depos; // to accept immediate value for ofs len
    },
    ValueType::U32 {
        /// Basic unary operators for `U32` IR registers (`l` suffix).
        ///
        /// Notable ones:
        /// - Movl: duplicate register (as in SSA paradigm)
        unary: Negl, Movl;
        /// Extract lower and higher 32 bits into 64 bit results.
        convert: Extrl, Extrh;
        /// Basic binary arithmetic operators for `U32` IR registers (`l` suffix).
        binary: Subl;
        /// Basic logical (bitwise) arithmetic operators for `U32` IR registers (`l` suffix).
        binary: Andl, Orl, Xorl, Andcl;
        /// Bit shift/rotations for `U32` IR registers (`l` suffix).
        binary: Sarl, Rotrl;
        /// Double add fused into a single instruction for `U32` IR registers.
        ///
        /// `[rh:rl] = [ah:al] + [bh:bl]`
        custom: Add2l, rl, rh, al, ah, bl, bh;
        override_maker: Movl;
    },
    ValueType::F64 {
        /// Basic unary operators for `F64` IR registers (`d` suffix).
        ///
        /// Notable ones:
        /// - Movd: duplicate register (as in SSA paradigm)
        unary: Movd;
        /// Basic binary arithmetic operators for `F64` IR registers (`d` suffix).
        binary: Addd, Subd, Muld, Divd;
        override_maker: Movd;
    }
}

// the bitfield is designed to support inverting condition or allowing equality
// with only a single bit toggle.
bitflags! {
    /// Condition codes for use in conditional operators.
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
    /// Encoding for different trap causes.
    pub struct TrapOp: u64 {
        /// The next block to execute is not known and requires lookup by the runtime.
        ///
        /// Value meaning: start address of the next block to be executed.
        const LOOKUP_TB = 0;
        /// The corresponding guest instruction is an undefined instruction.
        ///
        /// Value meaning: guest PC of the faulty instruction.
        const UNDEF_OPCODE = 1;
        /// The guest is attempting to perform an impossible memory access.
        ///
        /// Note that this only captures faulty addresses during the disassembly phase, e.g.
        /// destinations that are outside of the guest virtual memory space (see [`GUEST_SIZE`]()).
        ///
        /// Value meaning: guest address of the faulty memory access.
        const ACCESS_FAULT = 2;
        /// The guest is attempting to perform a system call.
        ///
        /// Value meaning: host pointer to the syscall data block.
        const SYSCALL = 3;
        /// The guest is attempting to perform a dynamically-linked function call.
        ///
        /// Value meaning: guest address of the dynamic call site.
        const DYNAMIC = 4;
    }
}

impl Display for TrapOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let s = match *self {
            TrapOp::LOOKUP_TB => "lookup_tb",
            TrapOp::UNDEF_OPCODE => "undef_opcode",
            TrapOp::ACCESS_FAULT => "access_fault",
            TrapOp::SYSCALL => "syscall",
            TrapOp::DYNAMIC => "dynamic",
            _ => unreachable!(),
        };

        write!(f, "{}", s)
    }
}

// optimizations when creating Op
mod opt;
// fused Ops or those with a different interface
mod meta;
