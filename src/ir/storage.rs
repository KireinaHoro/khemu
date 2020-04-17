extern crate num_traits;

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{Debug, Display, Error, Formatter};
use std::hash::{Hash, Hasher};

// trait for host storage assignment
pub trait HostStorage: Default + Display {}

// valid value types
#[derive(Debug, PartialEq)]
pub enum ValueType {
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
    storage: RefCell<R>,
}

impl<R: HostStorage> KHVal<R> {
    // allocate value with unassigned storage
    pub fn new(ty: ValueType) -> Self {
        Self {
            ty,
            storage: RefCell::new(Default::default()),
        }
    }
}

impl<R: HostStorage> Display for KHVal<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut s = DefaultHasher::new();
        (self as *const Self as u64).hash(&mut s);
        write!(
            f,
            "<#{1:07x}, {2}, {0}>",
            self.storage.borrow(),
            s.finish() % 0x10000000,
            self.ty
        )
    }
}
