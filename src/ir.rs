extern crate num_traits;
extern crate paste;

use num_traits::Num;
use std::marker::PhantomData;

macro_rules! gen_op_emit {
    ( $mnemonic:ident, $valtype:ident, $casttype:ident, [ $( $op:ident : $type:ident ),* ] ) => {
        paste::item! {
            impl<$valtype: Num, $casttype: Num>
                [< Emit $mnemonic >] <$valtype, $casttype> for DumpIREmitter<$valtype, $casttype> {
                fn [< emit_ $mnemonic:snake >] (&mut self, $($op: &KHVal<$type>),*) {
                    self.contents.push_str(format!("{}\t{}\n",
                        stringify!([< $mnemonic:snake >]),
                        stringify!($($op:$type),*)).as_str());  // TODO(jsteward) find a proper way to represent KHVal in string
                }
            }
            impl<'a, $valtype: Num, $casttype: Num> $mnemonic<'a, $valtype, $casttype> {
                pub fn emit<EM>(emitter: &mut EM, $($op: &'a KHVal<$type>),*)
                where EM: Emitter<ValType=$valtype, CastSrc=$casttype>
                {
                    emitter.[< emit_ $mnemonic:snake >]($($op),*)
                }
            }
        }
    };
    ( $mnemonic:ident, $valtype:ident, [ $( $op:ident : $type:ident ),* ] ) => {
        paste::item! {
            impl<$valtype: Num, C: Num>
                [< Emit $mnemonic >] <$valtype> for DumpIREmitter<$valtype, C> {
                fn [< emit_ $mnemonic:snake >] (&mut self, $($op: &KHVal<$type>),*) {
                    self.contents.push_str(format!("{}\t{}\n",
                        stringify!([< $mnemonic:snake >]),
                        stringify!($($op:$type),*)).as_str());  // TODO(jsteward) find a proper way to represent KHVal in string
                }
            }
            impl<'a, $valtype: Num> $mnemonic<'a, $valtype> {
                pub fn emit<EM>(emitter: &mut EM, $($op: &'a KHVal<$type>),*)
                where EM: Emitter<ValType=$valtype>
                {
                    emitter.[< emit_ $mnemonic:snake >]($($op),*)
                }
            }
        }
    }
}

macro_rules! gen_multi_type_op {
    ( $mnemonic:ident, [ $($generic:ident),+ ], $( $op:ident : $type:ident ),* ) => {
        #[allow(dead_code)]
        #[derive(Debug)]
        pub struct $mnemonic<'a, $($generic: Num),*> {
            $( $op: &'a KHVal<$type> ),*
        }

        gen_op_emit!($mnemonic, $($generic),*, [ $($op: $type),* ]);

        paste::item! {
            pub trait [< Emit $mnemonic >] <$($generic: Num),*> {
                fn [< emit_ $mnemonic:snake >] (&mut self, $($op: &KHVal<$type>),*);
            }
        }
    }
}

macro_rules! gen_single_type_op {
    ( $mnemonic:ident, $( $op:ident ),* ) => {
        gen_multi_type_op!($mnemonic, [T], $( $op: T ),*);
    }
}

// used to dump IR into string form
pub struct DumpIREmitter<V: Num, C: Num> {
    pub contents: String,
    p1: PhantomData<V>,
    p2: PhantomData<C>,
}

impl<V: Num, C: Num> DumpIREmitter<V, C> {
    pub fn new() -> Self {
        Self {
            contents: String::new(),
            p1: PhantomData,
            p2: PhantomData,
        }
    }
}

impl<V: Num, C: Num> Emitter for DumpIREmitter<V, C> {
    type ValType = V;
    type CastSrc = C;
}

macro_rules! gen_op {
    ( conv: $($mnemonic:ident),* ) => {
        $( gen_multi_type_op!($mnemonic, [T, U], rd: T, rs1: U); )*

        paste::item! {
            pub trait EmitConv<T: Num, U: Num>: $( [< Emit $mnemonic >] <T, U> + )* {}

            impl<T: Num, U: Num> EmitConv<T, U> for DumpIREmitter<T, U> {}
        }
    };
    ( unary: $($mnemonic:ident),* ) => {
        $( gen_single_type_op!($mnemonic, rd, rs1); )*

        paste::item! {
            pub trait EmitUnary<T: Num>: $( [< Emit $mnemonic >] <T> + )* {}

            impl<T: Num, U: Num> EmitUnary<T> for DumpIREmitter<T, U> {}
        }
    };
    ( binary: $($mnemonic:ident),* ) => {
        $( gen_single_type_op!($mnemonic, rd, rs1, rs2); )*

        paste::item! {
            pub trait EmitBinary<T: Num>: $( [< Emit $mnemonic >] <T> + )* {}

            impl<T: Num, U: Num> EmitBinary<T> for DumpIREmitter<T, U> {}
        }
    };
    ( custom: $( $mnemonic:ident, $($arg:ident),* );* ) => {
        $( gen_single_type_op!($mnemonic, $($arg),*); )*

        paste::item! {
            pub trait EmitCustom<T: Num>: $( [< Emit $mnemonic >] <T> + )* {}

            impl<T: Num, U: Num> EmitCustom<T> for DumpIREmitter<T, U> {}
        }
    }
}

#[rustfmt::skip]
gen_op!(conv:
    Cast
);
#[rustfmt::skip]
gen_op!(unary:
    Neg, Not, Mov, Bswap, ExtU, ExtS
);
#[rustfmt::skip]
gen_op!(binary:
    Add, Sub, Mul, Div, Rem, Remu, // arithmetic
    And, Or, Xor, Andc, Eqv, Nand, Nor, Orc, Clz, Ctz, // logical
    Shl, Shr, Sar, Rotl, Rotr // shifts / rotates
);
#[rustfmt::skip]
gen_op!(custom:
    Call, rd, func, rs1, rs2, rs3, rs4
);

// host-specific emitter
pub trait Emitter:
    EmitConv<<Self as Emitter>::ValType, <Self as Emitter>::CastSrc>
    + EmitUnary<<Self as Emitter>::ValType>
    + EmitBinary<<Self as Emitter>::ValType>
    + EmitCustom<<Self as Emitter>::ValType>
{
    type ValType: Num;
    type CastSrc: Num;
}

// before register assignment
#[derive(Default, Debug, Hash)]
pub struct KHVal<U: Num> {
    phantom: PhantomData<U>,
}

// TODO(jsteward) implement proper allocation and release semantics
impl<U: Num> KHVal<U> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
