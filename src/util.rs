extern crate num_traits;

use num_traits::int;
use std::mem;

#[inline]
pub fn extract<T>(val: T, start: usize, len: usize) -> T
where
    T: int::PrimInt,
{
    let word_bits = 8 * mem::size_of::<T>();
    (val >> start) & (!T::zero() >> (word_bits - len))
}

#[inline]
pub fn sextract<T>(val: T, start: usize, len: usize) -> T
where
    T: int::PrimInt,
{
    let word_bits = 8 * mem::size_of::<T>();
    ((val << (word_bits - len - start)) as T) >> (word_bits - len)
}
