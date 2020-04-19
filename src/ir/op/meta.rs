use super::*;

impl<R: HostStorage> Op<R> {
    pub fn push_load<C: GuestContext<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        addr: &Rc<KHVal<R>>,
        mem_op: MemOp,
    ) {
        assert_eq!(rd.ty, ValueType::U64);
        assert_eq!(addr.ty, ValueType::U64);
        let mem_op = ctx.alloc_u64(mem_op.bits());
        Op::_push_load(ctx, rd, addr, &mem_op);
    }

    pub fn push_store<C: GuestContext<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        addr: &Rc<KHVal<R>>,
        mem_op: MemOp,
    ) {
        assert_eq!(rd.ty, ValueType::U64);
        assert_eq!(addr.ty, ValueType::U64);
        let mem_op = ctx.alloc_u64(mem_op.bits());
        Op::_push_store(ctx, rd, addr, &mem_op);
    }

    pub fn push_setc<C: GuestContext<R>>(
        ctx: &mut C,
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
