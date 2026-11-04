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
