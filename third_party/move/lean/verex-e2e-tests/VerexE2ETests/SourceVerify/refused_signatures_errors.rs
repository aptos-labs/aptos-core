// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A signature the unit cannot state makes that function opaque: it is reported, its callers
// are refused, and the rest of the crate verifies.

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

pub struct W<I>(pub I);

impl<I: Iterator> Iterator for W<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

pub fn projection<T: Tr>(x: T::Out) -> T::Out {
    x
}

pub fn in_option<I: Iterator>(mut i: I) -> Option<I::Item> {
    i.next()
}

pub fn struct_field(h: &Field<A>) -> u64 {
    h.value
}

pub fn takes_float(x: f32) -> u32 {
    x as u32
}

pub fn returns_pointer(value: &u32) -> *const u32 {
    value as *const u32
}

pub fn const_parameter<const N: usize>(values: [u64; N]) -> u64 {
    values[0]
}

pub fn calls_projection(a: u64) -> u64 {
    projection::<A>(a)
}

pub fn calls_with_contract(a: u64) -> u64 {
    projection::<A>(a)
}

pub fn calls_const_parameter(a: u64) -> u64 {
    const_parameter([a])
}

pub fn calls_a_refused_caller(a: u64) -> u64 {
    calls_with_contract(a)
}

pub fn plain(a: u64) -> u64 {
    a / 2
}
