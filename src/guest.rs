// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

/// The ARM64 frontend.
pub mod arm64;

use crate::host::HostContext;
use crate::ir::op::Op;
use crate::ir::storage::*;
use bitflags::_core::fmt::{Error, Formatter};
use std::fmt::Display;
use std::rc::{Rc, Weak};

/// Reason that the current translation block terminated.
pub enum DisasException {
    /// Direct continue to the next instruction.
    ///
    /// Possible cases:
    /// - jump target (forcing start of a new block)
    /// - translation block size limit exceeded
    ///
    /// Fields:
    /// - `0`: the next PC
    Continue(usize),
    /// Branch, both conditional or unconditional.
    ///
    /// Possible cases:
    /// - direct branch (and link)
    /// - conditional branch
    ///
    /// Fields (`None` denotes unknown target at compile time)
    /// - `0`: PC for branch taken
    /// - `1`: PC for branch not taken
    Branch(Option<usize>, Option<usize>),
    /// Unexpected error.
    ///
    /// Possible cases:
    /// - unimplemented function
    ///
    /// Fields:
    /// - `0`: the error message
    Unexpected(String),
}

impl Display for DisasException {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            DisasException::Continue(d) => write!(f, "direct continue, next instr: {:#x}", d),
            DisasException::Branch(d, a) => {
                let df = if let Some(a) = d {
                    format!("{:#x}", a)
                } else {
                    "unknown".to_owned()
                };
                let af = if let Some(a) = a {
                    format!("{:#x}", a)
                } else {
                    "unknown".to_owned()
                };
                write!(f, "branch: direct {}; aux {}", df, af)
            }
            DisasException::Unexpected(s) => write!(f, "unexpected: {}", s),
        }
    }
}

/// Denotes a disassembled, but not yet emitted, IR block.
pub struct TranslationBlock<R: HostStorage> {
    /// Start address in guest view.
    pub start_pc: usize,
    /// Generated IR operations.
    pub ops: Vec<Op<R>>,
    direct_chain_idx: Option<usize>, // taken branch
    aux_chain_idx: Option<usize>,    // not taken branch
}

/// Disassembler functions to be invoked from the runtime.
pub trait Disassembler<R: HostStorage> {
    /// Run disassembly loop to generate a translation block.
    fn disas_block(&mut self, start_pos: usize, tb_size: usize) -> DisasException;

    /// Retrieve the newly-generated translation block.
    fn get_tb(&mut self) -> TranslationBlock<R>;

    // get tracking weak pointers of allocated KHVals
    #[doc(hidden)]
    fn get_tracking(&self) -> &[Weak<KHVal<R>>];
    // run housekeeping on tracking
    #[doc(hidden)]
    fn clean_tracking(&mut self);
}

/// Disassembler functions to be invoked from within the frontend.
pub trait DisasContext<R: HostStorage>: Disassembler<R>
where
    Self: Sized,
{
    /// PC of last fetched instruction.
    fn curr_pc(&self) -> usize;
    /// PC of upcoming instruction.
    fn next_pc(&self) -> usize;

    /// Allocate a new unassigned [KHVal](../ir/storage/struct.KHVal.html).
    fn alloc_val(&mut self, ty: ValueType) -> Rc<KHVal<R>>;
    /// Allocate a new label.
    fn alloc_label(&mut self) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::Label);
        *ret.storage.borrow_mut() = R::HostContext::get().make_label();
        ret
    }
    /// Allocate `u32` immediate value.
    fn alloc_u32(&mut self, v: u32) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::U32);
        *ret.storage.borrow_mut() = R::HostContext::get().make_u32(v);
        ret
    }
    /// Allocate `u64` immediate value.
    fn alloc_u64(&mut self, v: u64) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::U64);
        *ret.storage.borrow_mut() = R::HostContext::get().make_u64(v);
        ret
    }
    /// Allocate `f64` immediate value.
    fn alloc_f64(&mut self, v: f64) -> Rc<KHVal<R>> {
        let ret = self.alloc_val(ValueType::F64);
        *ret.storage.borrow_mut() = R::HostContext::get().make_f64(v);
        ret
    }

    /// Push an Op into the current translation block.
    ///
    /// This should queue the Op pending for next translation block whenever there is an exception.
    fn push_op(&mut self, op: Op<R>);
}
