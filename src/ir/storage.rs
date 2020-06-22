// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

extern crate num_traits;

use crate::host::HostContext;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{Debug, Display, Error, Formatter};
use std::hash::{Hash, Hasher};

/// Underlying storage for IR registers.
pub trait HostStorage: Default + Display + PartialEq {
    /// Type of the corresponding host context.
    type HostContext: HostContext<StorageType = Self> + 'static;

    /// Attempt to cast the storage to constant `u32` for constant propagation.
    fn try_as_u32(&self) -> Option<u32>;
    /// Attempt to cast the storage to constant `u64` for constant propagation.
    fn try_as_u64(&self) -> Option<u64>;
    /// Attempt to cast the storage to constant `f64` for constant propagation.
    fn try_as_f64(&self) -> Option<f64>;
}

/// Valid value types for an IR register.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ValueType {
    /// Fake value useful for dealing with jump targets.
    Label,
    /// 32bit word (`l` suffix in operators)
    U32,
    /// 64bit word (no suffix in operators)
    U64,
    /// Double word (`d` suffix in operators)
    F64,
}

impl Display for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        Debug::fmt(self, f)
    }
}

/// IR register in SSA form.
#[derive(Debug)]
pub struct KHVal<R: HostStorage> {
    /// Data type of the register.
    pub ty: ValueType,
    /// Backend storage of the register.
    pub storage: RefCell<R>,
}

impl<R: HostStorage> KHVal<R> {
    /// Allocate new unassigned register from the frontend.
    pub fn new(ty: ValueType) -> Self {
        Self {
            ty,
            storage: RefCell::new(Default::default()),
        }
    }

    /// Allocate new named register from the frontend.
    pub fn named(name: String, ty: ValueType) -> Self {
        Self {
            ty,
            storage: RefCell::new(R::HostContext::get().make_named(name, ty)),
        }
    }

    /// Allocate new label from the frontend.
    pub fn label() -> Self {
        Self {
            ty: ValueType::Label,
            storage: RefCell::new(R::HostContext::get().make_label()),
        }
    }

    /// Allocate `U32` immediate value from the frontend.
    pub fn u32(v: u32) -> Self {
        Self {
            ty: ValueType::U32,
            storage: RefCell::new(R::HostContext::get().make_u32(v)),
        }
    }

    /// Allocate `U64` immediate value from the frontend.
    pub fn u64(v: u64) -> Self {
        Self {
            ty: ValueType::U64,
            storage: RefCell::new(R::HostContext::get().make_u64(v)),
        }
    }

    /// Allocate `F64` immediate value from the frontend.
    pub fn f64(v: f64) -> Self {
        Self {
            ty: ValueType::F64,
            storage: RefCell::new(R::HostContext::get().make_f64(v)),
        }
    }
}

impl<R: HostStorage> Display for KHVal<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut s = DefaultHasher::new();
        (self as *const Self as u64).hash(&mut s);
        // we hope that 5 digits are enough for display purposes
        if let Err(_) = write!(f, "{}", self.storage.borrow()) {
            write!(f, "%{:05x}", s.finish() % 0x100000)
        } else {
            Ok(())
        }
    }
}

bitflags! {
    /// Used in Load and Store ops to denote the exact memory operation.
    pub struct MemOp: u64 {
        const SIZE_8    = 0b00;
        const SIZE_16   = 0b01;
        const SIZE_32   = 0b10;
        const SIZE_64   = 0b11;
        const SIZE_MASK = 0b11;

        const UNSIGNED    = 0;
        const SIGN_EXTEND = 0b1 << 2;
        const BYTE_SWAP   = 0b1 << 3;

        // no align        0b000 << 4;
        const ALIGN_2    = 0b001 << 4;
        const ALIGN_4    = 0b010 << 4;
        const ALIGN_8    = 0b011 << 4;
        const ALIGN_16   = 0b100 << 4;
        const ALIGN_32   = 0b101 << 4;
        const ALIGN_64   = 0b110 << 4;
        const ALIGN_MASK = 0b111 << 4;

        // aliases for operand types
        const UB = Self::SIZE_8.bits;
        const UW = Self::SIZE_16.bits;
        const UL = Self::SIZE_32.bits;
        const SB = Self::SIZE_8.bits | Self::SIGN_EXTEND.bits;
        const SW = Self::SIZE_16.bits | Self::SIGN_EXTEND.bits;
        const SL = Self::SIZE_32.bits | Self::SIGN_EXTEND.bits;
        const Q = Self::SIZE_64.bits;
        // aliases assuming a little-endian host
        const GUEST_LE  = 0;  // no need for byte swap
        const GUEST_BE  = Self::BYTE_SWAP.bits;
    }
}

impl MemOp {
    /// Construct `MemOp` from memory size.
    ///
    /// Only access sizes of 1, 2, 4, and 8 are supported.
    pub fn from_size(bytes: u64) -> Self {
        match bytes {
            1 => Self::SIZE_8,
            2 => Self::SIZE_16,
            4 => Self::SIZE_32,
            8 => Self::SIZE_64,
            _ => unreachable!("size of {} bytes not supported", bytes),
        }
    }

    /// Construct `MemOp` from signedness.
    pub fn from_sign(sign: bool) -> Self {
        if sign {
            Self::SIGN_EXTEND
        } else {
            Self::UNSIGNED
        }
    }

    /// Retrieve the access size from `MemOp`.
    pub fn get_size(&self) -> u64 {
        match *self & Self::SIZE_MASK {
            Self::SIZE_8 => 1,
            Self::SIZE_16 => 2,
            Self::SIZE_32 => 4,
            Self::SIZE_64 => 8,
            _ => unreachable!(),
        }
    }

    /// Retrieve the signedness from `MemOp`.
    pub fn get_sign(&self) -> bool {
        (*self & Self::SIGN_EXTEND).bits != 0
    }
}
