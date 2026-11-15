// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A signature the unit cannot state makes that function opaque, and the others stand.

pub trait Tr {
    type Out;
}

pub struct A;

impl Tr for A {
    type Out = u64;
}

pub struct Field<T: Tr> {
    pub value: T::Out,
}

pub fn projection<T: Tr>(x: T::Out) -> T::Out {
    x
}

pub fn struct_field(h: &Field<A>) -> u64 {
    h.value
}

pub fn takes_float(x: f32) -> u32 {
    x as u32
}

pub fn const_parameter<const N: usize>(values: [u64; N]) -> u64 {
    values[0]
}

pub fn calls_projection(a: u64) -> u64 {
    projection::<A>(a)
}

pub fn holds_a_float_closure(x: u32) -> u32 {
    let f = |a: f32| a as u32;
    let _g = &f;
    x
}

pub fn plain(a: u64) -> u64 {
    a / 2
}
