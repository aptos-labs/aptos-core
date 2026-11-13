// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::convert::Infallible;
use std::marker::PhantomData;

pub struct Unit;

pub struct Nested(pub Unit, pub ());

pub struct Marker(pub PhantomData<u8>);

pub enum Single {
    Only,
}

pub fn none() -> Option<Infallible> {
    Option::<Infallible>::None
}

pub fn nested() -> Nested {
    const VALUE: Nested = Nested(Unit, ());
    VALUE
}

pub fn marker() -> Marker {
    Marker(PhantomData)
}

pub fn single() -> Single {
    const VALUE: Single = Single::Only;
    VALUE
}

#[inline(never)]
pub fn slice_length(bytes: &[u8]) -> usize {
    bytes.len()
}

pub fn empty_slice_length() -> usize {
    let bytes: [u8; 0] = [];
    slice_length(&bytes)
}

#[inline(never)]
pub fn slice_lengths(first: &[u8], second: &[u8]) -> usize {
    first.len() + second.len()
}

pub fn repeated_shared_empty() -> usize {
    let bytes: [u8; 0] = [];
    let first: &[u8] = &bytes;
    let second: &[u8] = &bytes;
    slice_lengths(first, second)
}

#[inline(never)]
pub fn mutable_slice_length(bytes: &mut [u8]) -> usize {
    bytes.len()
}

pub fn repeated_mutable_empty() -> usize {
    let mut bytes: [u8; 0] = [];
    let first = mutable_slice_length(&mut bytes);
    first + mutable_slice_length(&mut bytes)
}
