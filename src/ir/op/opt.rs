use super::*;

impl<R: HostStorage> Op<R> {
    pub fn push_add<C: DisasContext<R> + Disassembler<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        rs1: &Rc<KHVal<R>>,
        rs2: &Rc<KHVal<R>>,
    ) {
        if let Some(0) = rd.storage.borrow().try_as_u64() {
            return;
        }
        if let Some(0) = rs1.storage.borrow().try_as_u64() {
            Op::push_mov(ctx, rd, rs2);
            return;
        }
        if let Some(0) = rs2.storage.borrow().try_as_u64() {
            Op::push_mov(ctx, rd, rs1);
            return;
        }
        Op::_push_add(ctx, rd, rs1, rs2);
    }

    pub fn push_sub<C: DisasContext<R> + Disassembler<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        rs1: &Rc<KHVal<R>>,
        rs2: &Rc<KHVal<R>>,
    ) {
        if let Some(0) = rd.storage.borrow().try_as_u64() {
            return;
        }
        if let Some(0) = rs2.storage.borrow().try_as_u64() {
            Op::push_mov(ctx, rd, rs1);
            return;
        }
        Op::_push_sub(ctx, rd, rs1, rs2);
    }

    pub fn push_extuwq<C: DisasContext<R> + Disassembler<R>>(
        ctx: &mut C,
        rd: &Rc<KHVal<R>>,
        rs1: &Rc<KHVal<R>>,
    ) {
        if let Some(0) = rd.storage.borrow().try_as_u64() {
            return;
        }
        Op::_push_extuwq(ctx, rd, rs1);
    }
}
