// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_openapi::impl_poem_type;
use move_deps::move_core_types::identifier::{IdentStr, Identifier};

use serde::{Deserialize, Serialize};
use std::{
    convert::{From, Into},
    fmt,
    ops::Deref,
    str::FromStr,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct IdentifierWrapper(pub Identifier);

impl FromStr for IdentifierWrapper {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        Ok(IdentifierWrapper(Identifier::from_str(s)?))
    }
}

impl From<IdentifierWrapper> for Identifier {
    fn from(value: IdentifierWrapper) -> Identifier {
        value.0
    }
}

impl From<Identifier> for IdentifierWrapper {
    fn from(value: Identifier) -> IdentifierWrapper {
        Self(value)
    }
}

impl From<&IdentStr> for IdentifierWrapper {
    fn from(ident_str: &IdentStr) -> Self {
        Self(Identifier::from(ident_str))
    }
}

impl AsRef<IdentStr> for IdentifierWrapper {
    fn as_ref(&self) -> &IdentStr {
        self.0.as_ref()
    }
}

impl Deref for IdentifierWrapper {
    type Target = IdentStr;

    fn deref(&self) -> &IdentStr {
        self.0.deref()
    }
}

impl fmt::Display for IdentifierWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Identifier::fmt(&self.0, f)
    }
}

impl_poem_type!(IdentifierWrapper);
