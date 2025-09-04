// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

/// A wrapper around a `String` representing a named address.
///
/// A valid named address is an identifier that
/// - Begins with an ASCII letter (`a–z`, `A–Z`) or an underscore (`_`)
/// - Contains only ASCII letters, digits (`0–9`) or underscores (`_`)
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct NamedAddress(String);

impl Deref for NamedAddress {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl DerefMut for NamedAddress {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_str()
    }
}

impl AsRef<str> for NamedAddress {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsMut<str> for NamedAddress {
    fn as_mut(&mut self) -> &mut str {
        self.0.as_mut_str()
    }
}

impl Serialize for NamedAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

fn is_valid_named_address(s: &str) -> bool {
    let mut chars = s.chars();

    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
        _ => return false,
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl<'de> Deserialize<'de> for NamedAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let make_err = || {
            serde::de::Error::custom("Invalid named address -- must start with a letter or _; only letters, digits, and _ allowed.")
        };

        let s = String::deserialize(deserializer).map_err(|_| make_err())?;

        if !is_valid_named_address(&s) {
            return Err(make_err());
        }

        Ok(Self(s))
    }
}

impl Display for NamedAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

impl Debug for NamedAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
