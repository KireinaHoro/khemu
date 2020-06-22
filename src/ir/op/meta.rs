// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;

use log::*;

impl<R: HostStorage> Op<R> {
    pub fn push_load(
        ctx: &mut impl DisasContext<R>,
        rd: &Rc<KHVal<R>>,
        addr: &Rc<KHVal<R>>,
        mem_op: MemOp,
    ) {
        assert_eq!(rd.ty, ValueType::U64);
        assert_eq!(addr.ty, ValueType::U64);
        let mem_op = ctx.alloc_u64(mem_op.bits());
        Op::_push_load(ctx, rd, addr, &mem_op);
    }

    pub fn push_store(
        ctx: &mut impl DisasContext<R>,
        rd: &Rc<KHVal<R>>,
        addr: &Rc<KHVal<R>>,
        mem_op: MemOp,
    ) {
        assert_eq!(rd.ty, ValueType::U64);
        assert_eq!(addr.ty, ValueType::U64);
        let mem_op = ctx.alloc_u64(mem_op.bits());
        Op::_push_store(ctx, rd, addr, &mem_op);
    }

    pub fn push_setc(
        ctx: &mut impl DisasContext<R>,
        rd: &Rc<KHVal<R>>,
        c1: &Rc<KHVal<R>>,
        c2: &Rc<KHVal<R>>,
        cc: CondOp,
    ) {
        let cc = ctx.alloc_u64(cc.bits());
        assert_eq!(c1.ty, c2.ty);
        // we can't use the default impl due to type violations
        ctx.push_op(match rd.ty {
            ValueType::U64 | ValueType::U32 => Op::Setc {
                rd: Rc::clone(rd),
                c1: Rc::clone(c1),
                c2: Rc::clone(c2),
                cc,
            },
            _ => unreachable!("setc only accepts u32 and u64 destination"),
        });
    }

    pub fn push_movc(
        ctx: &mut impl DisasContext<R>,
        rd: &Rc<KHVal<R>>,
        rs1: &Rc<KHVal<R>>,
        rs2: &Rc<KHVal<R>>,
        c1: &Rc<KHVal<R>>,
        c2: &Rc<KHVal<R>>,
        cc: CondOp,
    ) {
        trace!("push_movc");
        let cc = ctx.alloc_u64(cc.bits());
        assert_eq!(c1.ty, c2.ty);
        assert_eq!(rd.ty, rs1.ty);
        assert_eq!(rd.ty, rs2.ty);
        // we can't use the default impl due to type violations
        ctx.push_op(match rd.ty {
            ValueType::U64 | ValueType::U32 => Op::Movc {
                rd: Rc::clone(rd),
                rs1: Rc::clone(rs1),
                rs2: Rc::clone(rs2),
                c1: Rc::clone(c1),
                c2: Rc::clone(c2),
                cc,
            },
            _ => unreachable!("movc only accepts u32 and u64 destination"),
        });
    }

    pub fn push_brc(
        ctx: &mut impl DisasContext<R>,
        dest: &Rc<KHVal<R>>,
        c1: &Rc<KHVal<R>>,
        c2: &Rc<KHVal<R>>,
        cc: CondOp,
    ) {
        trace!("push_brc");
        let cc = ctx.alloc_u64(cc.bits());
        assert_eq!(c1.ty, c2.ty);
        assert_eq!(dest.ty, ValueType::Label);
        // we can't use the default impl due to type violations
        ctx.push_op(Op::Brc {
            dest: Rc::clone(dest),
            c1: Rc::clone(c1),
            c2: Rc::clone(c2),
            cc,
        });
    }

    pub fn push_extr(
        ctx: &mut impl DisasContext<R>,
        lo: &Rc<KHVal<R>>,
        hi: &Rc<KHVal<R>>,
        arg: &Rc<KHVal<R>>,
    ) {
        trace!("push_extr");
        Op::push_extrl(ctx, lo, arg);
        Op::push_extrh(ctx, hi, arg);
    }

    pub fn push_extru(
        ctx: &mut impl DisasContext<R>,
        rd: &Rc<KHVal<R>>,
        rs: &Rc<KHVal<R>>,
        ofs: u64,
        len: u64,
    ) {
        trace!("push_extru");
        let ofs = ctx.alloc_u64(ofs);
        let len = ctx.alloc_u64(len);
        Op::_push_extru(ctx, rd, rs, &ofs, &len);
    }

    pub fn push_extrs(
        ctx: &mut impl DisasContext<R>,
        rd: &Rc<KHVal<R>>,
        rs: &Rc<KHVal<R>>,
        ofs: u64,
        len: u64,
    ) {
        trace!("push_extrs");
        let ofs = ctx.alloc_u64(ofs);
        let len = ctx.alloc_u64(len);
        Op::_push_extrs(ctx, rd, rs, &ofs, &len);
    }

    pub fn push_depos(
        ctx: &mut impl DisasContext<R>,
        rd: &Rc<KHVal<R>>,
        rs1: &Rc<KHVal<R>>,
        rs2: &Rc<KHVal<R>>,
        ofs: u64,
        len: u64,
    ) {
        trace!("push_depos");
        let ofs = ctx.alloc_u64(ofs);
        let len = ctx.alloc_u64(len);
        Op::_push_depos(ctx, rd, rs1, rs2, &ofs, &len);
    }

    pub fn push_mov(ctx: &mut impl DisasContext<R>, rd: &Rc<KHVal<R>>, rs: &Rc<KHVal<R>>) {
        assert_eq!(rd.ty, rs.ty);
        trace!("push_mov");
        if rd.storage == rs.storage {
            trace!("rd == rs, do nothing");
            return;
        }
        match rd.ty {
            ValueType::U64 => Op::_push_mov(ctx, rd, rs),
            ValueType::U32 => Op::_push_movl(ctx, rd, rs),
            ValueType::F64 => Op::_push_movd(ctx, rd, rs),
            _ => unreachable!(),
        }
    }

    pub fn push_trap(ctx: &mut impl DisasContext<R>, cause: TrapOp, val: &Rc<KHVal<R>>) {
        trace!("push_trap");
        let cause = ctx.alloc_u64(cause.bits);
        let new_val = ctx.alloc_val(val.ty);
        Op::push_mov(ctx, &new_val, val);
        Op::_push_trap(ctx, &cause, &new_val);
    }
}
