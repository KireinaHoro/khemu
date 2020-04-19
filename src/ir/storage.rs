extern crate num_traits;

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{Debug, Display, Error, Formatter};
use std::hash::{Hash, Hasher};

// trait for host storage assignment
// all implementers must provide support for immediate numbers
// a real host storage will probably support registers and memory as well
pub trait HostStorage: Default + Display {
    fn make_u32(v: u32) -> Self;
    fn make_u64(v: u64) -> Self;
    fn make_f64(v: f64) -> Self;
    // used to create named data, such as guest fixed registers
    fn make_named(name: String) -> Self;
}

// valid value types
#[derive(Debug, PartialEq)]
pub enum ValueType {
    U32,
    U64,
    F64,
}

impl Display for ValueType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        Debug::fmt(self, f)
    }
}

// Denotes a value used in IR.  May correspond to
#[derive(Debug)]
pub struct KHVal<R: HostStorage> {
    pub ty: ValueType,
    pub storage: RefCell<R>,
}

impl<R: HostStorage> KHVal<R> {
    // allocate value with unassigned storage
    pub fn new(ty: ValueType) -> Self {
        Self {
            ty,
            storage: RefCell::new(Default::default()),
        }
    }

    pub fn named(name: String, ty: ValueType) -> Self {
        Self {
            ty,
            storage: RefCell::new(R::make_named(name)),
        }
    }

    // used to construct U32 value
    pub fn u32(v: u32) -> Self {
        Self {
            ty: ValueType::U32,
            storage: RefCell::new(R::make_u32(v)),
        }
    }

    // used to construct U64 value
    pub fn u64(v: u64) -> Self {
        Self {
            ty: ValueType::U64,
            storage: RefCell::new(R::make_u64(v)),
        }
    }

    // used to construct F64 value
    pub fn f64(v: f64) -> Self {
        Self {
            ty: ValueType::F64,
            storage: RefCell::new(R::make_f64(v)),
        }
    }
}

impl<R: HostStorage> Display for KHVal<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut s = DefaultHasher::new();
        (self as *const Self as u64).hash(&mut s);
        // we hope that 5 digits are enough for display purposes
        write!(
            f,
            "<#{1:05x}, {0}>",
            self.storage.borrow(),
            s.finish() % 0x100000,
        )
    }
}

// used in Load and Store ops to denote exact memory operation
bitflags! {
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
    pub fn from_size(bytes: u64) -> Self {
        match bytes {
            1 => Self::SIZE_8,
            2 => Self::SIZE_16,
            4 => Self::SIZE_32,
            8 => Self::SIZE_64,
            _ => unreachable!("size of {} bytes not supported", bytes),
        }
    }

    pub fn from_sign(sign: bool) -> Self {
        if sign {
            Self::SIGN_EXTEND
        } else {
            Self::UNSIGNED
        }
    }
}
