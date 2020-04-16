extern crate num_traits;

use std::marker::PhantomData;

// Denotes a value used in IR.  May correspond to
#[derive(Default, Debug, Hash)]
pub struct KHVal<T> {
    // TODO(jsteward) fill in register assignment logic
    phantom: PhantomData<T>,
}

// TODO(jsteward) implement proper allocation and release semantics
impl<T> KHVal<T> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
