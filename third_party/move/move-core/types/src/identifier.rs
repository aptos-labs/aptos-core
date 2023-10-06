// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! An identifier is the name of an entity (module, resource, function, etc) in Move.
//!
//! A valid identifier consists of an ASCII string which satisfies any of the conditions:
//!
//! * The first character is a letter and the remaining characters are letters, digits or
//!   underscores.
//! * The first character is an underscore, and there is at least one further letter, digit or
//!   underscore.
//!
//! The spec for allowed identifiers is similar to Rust's spec
//! ([as of version 1.38](https://doc.rust-lang.org/1.38.0/reference/identifiers.html)).
//!
//! Allowed identifiers are currently restricted to ASCII due to unresolved issues with Unicode
//! normalization. See [Rust issue #55467](https://github.com/rust-lang/rust/issues/55467) and the
//! associated RFC for some discussion. Unicode identifiers may eventually be supported once these
//! issues are worked out.
//!
//! This module only determines allowed identifiers at the bytecode level. Move source code will
//! likely be more restrictive than even this, with a "raw identifier" escape hatch similar to
//! Rust's `r#` identifiers.
//!
//! Among other things, identifiers are used to:
//! * specify keys for lookups in storage
//! * do cross-module lookups while executing transactions

use crate::intern_table::{InstanceUID, InstanceUniverse};
use anyhow::{bail, Result};
#[cfg(any(test, feature = "fuzzing"))]
use arbitrary::Unstructured;
use once_cell::sync::Lazy;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, hash::Hash, str::FromStr};

/// Return true if this character can appear in a Move identifier.
///
/// Note: there are stricter restrictions on whether a character can begin a Move
/// identifier--only alphabetic characters are allowed here.
#[inline]
pub const fn is_valid_identifier_char(c: char) -> bool {
    matches!(c, '_' | 'a'..='z' | 'A'..='Z' | '0'..='9')
}

/// Returns `true` if all bytes in `b` after the offset `start_offset` are valid
/// ASCII identifier characters.
const fn all_bytes_valid(b: &[u8], start_offset: usize) -> bool {
    let mut i = start_offset;
    // TODO(philiphayes): use for loop instead of while loop when it's stable in const fn's.
    while i < b.len() {
        if !is_valid_identifier_char(b[i] as char) {
            return false;
        }
        i += 1;
    }
    true
}

/// Describes what identifiers are allowed.
///
/// For now this is deliberately restrictive -- we would like to evolve this in the future.
// TODO: "<SELF>" is coded as an exception. It should be removed once CompiledScript goes away.
// Note: needs to be pub as it's used in the `Identifier::new` macro.
pub const fn is_valid(s: &str) -> bool {
    // Rust const fn's don't currently support slicing or indexing &str's, so we
    // have to operate on the underlying byte slice. This is not a problem as
    // valid identifiers are (currently) ASCII-only.
    let b = s.as_bytes();
    match b {
        b"<SELF>" => true,
        [b'a'..=b'z', ..] | [b'A'..=b'Z', ..] => all_bytes_valid(b, 1),
        [b'_', ..] if b.len() > 1 => all_bytes_valid(b, 1),
        _ => false,
    }
}

/// A regex describing what identifiers are allowed. Used for proptests.
// TODO: "<SELF>" is coded as an exception. It should be removed once CompiledScript goes away.
#[cfg(any(test, feature = "fuzzing"))]
#[allow(dead_code)]
pub(crate) static ALLOWED_IDENTIFIERS: &str =
    r"(?:[a-zA-Z][a-zA-Z0-9_]*)|(?:_[a-zA-Z0-9_]+)|(?:<SELF>)";
#[cfg(any(test, feature = "fuzzing"))]
pub(crate) static ALLOWED_NO_SELF_IDENTIFIERS: &str =
    r"(?:[a-zA-Z][a-zA-Z0-9_]*)|(?:_[a-zA-Z0-9_]+)";

pub static IDENTIFIER_UNIVERSE: Lazy<InstanceUniverse<String>> = Lazy::new(InstanceUniverse::new);

pub type IdentifierID = InstanceUID<'static, String>;

/// An owned identifier.
///
/// For more details, see the module level documentation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
// #[cfg_attr(any(test, feature = "fuzzing"), derive(arbitrary::Arbitrary))]
pub struct Identifier(IdentifierID);

impl Serialize for Identifier {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.inner_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "Identifier")]
        struct RawIdentifier(String);

        let raw = RawIdentifier::deserialize(deserializer)?;

        Ok(Identifier::new(raw.0).unwrap())
    }
}

impl Identifier {
    /// Creates a new `Identifier` instance.
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let s = s.into();
        if Self::is_valid(&s) {
            Ok(Self(IDENTIFIER_UNIVERSE.get(s.to_string())))
        } else {
            bail!("Invalid identifier '{}'", s);
        }
    }

    /// Returns true if this string is a valid identifier.
    pub fn is_valid(s: impl AsRef<str>) -> bool {
        is_valid(s.as_ref())
    }

    /// Returns if this identifier is `<SELF>`.
    /// TODO: remove once we fully separate CompiledScript & CompiledModule.
    pub fn is_self(&self) -> bool {
        self.0.inner_ref().as_str() == "<SELF>"
    }

    /// Converts a vector of bytes to an `Identifier`.
    pub fn from_utf8(vec: Vec<u8>) -> Result<Self> {
        let s = String::from_utf8(vec)?;
        Self::new(s)
    }

    /// Creates a borrowed version of `self`.
    pub fn as_str(&self) -> &str {
        self.0.inner_ref().as_str()
    }

    /// Converts this `Identifier` into a `String`.
    ///
    /// This is not implemented as a `From` trait to discourage automatic conversions -- these
    /// conversions should not typically happen.
    pub fn into_string(self) -> String {
        self.0.inner_ref().as_ref().clone()
    }

    /// Converts this `Identifier` into a UTF-8-encoded byte sequence.
    pub fn into_bytes(self) -> Vec<u8> {
        self.into_string().into_bytes()
    }

    pub fn len(&self) -> usize {
        self.0.inner_ref().len()
    }
}

impl FromStr for Identifier {
    type Err = anyhow::Error;

    fn from_str(data: &str) -> Result<Self> {
        Self::new(data)
    }
}

impl<'a> From<&'a str> for Identifier {
    fn from(value: &'a str) -> Self {
        Identifier::new(value.to_string()).unwrap()
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.0.inner_ref())
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for Identifier {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): ()) -> Self::Strategy {
        ALLOWED_NO_SELF_IDENTIFIERS
            .prop_map(|s| {
                // Identifier::new will verify that generated identifiers are correct.
                Identifier::new(s).unwrap()
            })
            .boxed()
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl<'a> arbitrary::Arbitrary<'a> for Identifier {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        todo!();
    }
}
