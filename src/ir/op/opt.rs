// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

use super::*;

use log::*;

impl<R: HostStorage> Op<R> {
    pub fn push_add(
        ctx: &mut impl DisasContext<R>,
        rd: &Rc<KHVal<R>>,
        rs1: &Rc<KHVal<R>>,
        rs2: &Rc<KHVal<R>>,
    ) {
        trace!("push_add");
        if let Some(0) = rd.storage.borrow().try_as_u64() {
            trace!("rd is 0, do nothing");
            return;
        }
        if let Some(0) = rs1.storage.borrow().try_as_u64() {
            trace!("rs1 is 0, rd = rs2");
            Op::push_mov(ctx, rd, rs2);
            return;
        }
        if let Some(0) = rs2.storage.borrow().try_as_u64() {
            trace!("rs2 is 0, rd = rs1");
            Op::push_mov(ctx, rd, rs1);
            return;
        }
        Op::_push_add(ctx, rd, rs1, rs2);
    }

    pub fn push_sub(
        ctx: &mut impl DisasContext<R>,
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

    pub fn push_extulq(ctx: &mut impl DisasContext<R>, rd: &Rc<KHVal<R>>, rs1: &Rc<KHVal<R>>) {
        if let Some(0) = rd.storage.borrow().try_as_u64() {
            return;
        }
        Op::_push_extulq(ctx, rd, rs1);
    }
}
