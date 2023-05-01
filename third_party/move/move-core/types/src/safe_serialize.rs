// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Custom serializers which track recursion nesting with a thread local,
//! and otherwise delegate to the derived serializers.
//!
//! This is currently only implemented for type tags, but can be easily
//! generalized, as the the only type-tag specific thing is the allowed nesting.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cell::RefCell;

pub(crate) const MAX_TYPE_TAG_NESTING: u8 = 9;

thread_local! {
    static TYPE_TAG_DEPTH: RefCell<u8> = RefCell::new(0);
}

pub(crate) fn type_tag_recursive_serialize<S, T>(t: &T, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    use serde::ser::Error;

    TYPE_TAG_DEPTH.with(|depth| {
        let mut r = depth.borrow_mut();
        if *r >= MAX_TYPE_TAG_NESTING {
            // for testability, we allow one level more
            return Err(S::Error::custom(
                "type tag nesting exceeded during serialization",
            ));
        }
        *r += 1;
        Ok(())
    })?;
    let res = t.serialize(s);
    TYPE_TAG_DEPTH.with(|depth| {
        let mut r = depth.borrow_mut();
        *r -= 1;
    });
    res
}

pub(crate) fn type_tag_recursive_deserialize<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    use serde::de::Error;
    TYPE_TAG_DEPTH.with(|depth| {
        let mut r = depth.borrow_mut();
        // For testability, we allow to serialize one more level than deserialize.
        if *r >= MAX_TYPE_TAG_NESTING - 1 {
            return Err(D::Error::custom(
                "type tag nesting exceeded during deserialization",
            ));
        }
        *r += 1;
        Ok(())
    })?;
    let res = T::deserialize(d);
    TYPE_TAG_DEPTH.with(|depth| {
        let mut r = depth.borrow_mut();
        *r -= 1;
    });
    res
}
