// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    iter::{zip, Peekable, Zip},
    num::TryFromIntError,
    ops::Range,
};

// Two associated types of one name, normalizing to the same type.
pub fn zipped(n: u32) -> Zip<Range<u32>, Range<u32>> {
    zip(0..n, 0..n)
}

// An associated type on concrete types, the impl having no parameter.
pub fn narrow(value: u32) -> Result<u8, TryFromIntError> {
    u8::try_from(value)
}

// Nested under `Option` and a reference.
pub fn peeked(values: &mut Peekable<Range<u32>>) -> Option<&u32> {
    values.peek()
}

// An associated type at an unsized type.
pub fn borrowed_contents(value: &Option<Vec<u32>>) -> Option<&[u32]> {
    value.as_deref()
}
