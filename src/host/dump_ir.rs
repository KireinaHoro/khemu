// This is a dummy host that simply dumps the IR without executing them

extern crate num_traits;

use crate::host::*;
use crate::ir::DumpIREmitter;
use num_traits::Num;
use std::marker::PhantomData;

pub struct DumpIRHostContext<V: Num, C: Num> {
    pub emitter: DumpIREmitter<V, C>,
    p1: PhantomData<V>,
    p2: PhantomData<C>,
}

impl<V: Num, C: Num> DumpIRHostContext<V, C> {
    pub fn new() -> Self {
        Self {
            emitter: DumpIREmitter::new(),
            p1: PhantomData,
            p2: PhantomData,
        }
    }
}

impl<V: Num, C: Num> HostContext for DumpIRHostContext<V, C> {
    type EM = DumpIREmitter<V, C>;
    type RegType = V;
    fn get_emitter(&mut self) -> &mut Self::EM {
        &mut self.emitter
    }
}
