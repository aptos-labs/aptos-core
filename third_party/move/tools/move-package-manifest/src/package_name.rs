// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
};

/// A wrapper around a `String` representing the name of a Move package.
///
/// A valid package name must:
/// - Begin with an ASCII letter (`a–z`, `A–Z`) or an underscore (`_`)
/// - Contain only ASCII letters, digits (`0–9`), hyphens (`-`), or underscores (`_`)
///
/// TODO: The rules above are tentative and are subject to change if we find incompatibility
///       in production.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageName(String);

impl Deref for PackageName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl DerefMut for PackageName {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_str()
    }
}

impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl AsMut<str> for PackageName {
    fn as_mut(&mut self) -> &mut str {
        self.0.as_mut_str()
    }
}

impl Serialize for PackageName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

fn is_valid_package_name(s: &str) -> bool {
    let mut chars = s.chars();

    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
        _ => return false,
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl<'de> Deserialize<'de> for PackageName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let make_err = || {
            serde::de::Error::custom("Invalid package name -- must start with a letter or _; only letters, digits, - and _ allowed.")
        };

        let s = String::deserialize(deserializer)?;

        if !is_valid_package_name(&s) {
            return Err(make_err());
        }

        Ok(Self(s))
    }
}

impl Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl PackageName {
    pub fn new(s: impl Into<String>) -> anyhow::Result<Self> {
        let s: String = s.into();

        if !is_valid_package_name(&s) {
            bail!("Invalid package name {:?}", s);
        }

        Ok(Self(s))
    }
}
