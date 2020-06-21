// SPDX-FileCopyrightText: 2020 Pengcheng Xu <i@jsteward.moe>
//
// SPDX-License-Identifier: BSD-3-Clause

extern crate num_traits;

use num_traits::int::PrimInt;
use num_traits::sign::{Signed, Unsigned};
use std::mem;

// shifts rely on the type being correct to enforce sign extension behavior

#[inline]
pub fn extract<T>(val: T, start: usize, len: usize) -> T
where
    T: Unsigned + PrimInt,
{
    let word_bits = 8 * mem::size_of::<T>();
    (val >> start) & (!T::zero() >> (word_bits - len))
}

#[inline]
pub fn sextract<T>(val: T, start: usize, len: usize) -> T
where
    T: Signed + PrimInt,
{
    let word_bits = 8 * mem::size_of::<T>();
    ((val << (word_bits - len - start)) as T) >> (word_bits - len)
}
