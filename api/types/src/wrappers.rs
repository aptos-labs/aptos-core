// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! The purpose of this file is to define wrappers that we can use in the
//! endpoint handlers, specifically for accepting these types as parameters.
//! In Poem, it is not enough to impl FromStr for the types we want to use
//! as path parameters, as that does not describe anything about the input.
//! These wrappers say "I don't care" and use the impl_poem_type and
//! impl_poem_parameter macros to make it that we declare these inputs as
//! just strings, using the FromStr impl to parse the path param. They can
//! then be unpacked to the real type beneath.

use crate::{MoveStructTag, MoveType};

use anyhow::{anyhow, Result};
use aptos_openapi::{impl_poem_parameter, impl_poem_type};
use move_deps::move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::TypeTag,
};

use poem_openapi::{NewType, Union};
use serde::{Deserialize, Serialize};
use std::{
    convert::{From, Into, TryFrom},
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
impl_poem_parameter!(IdentifierWrapper);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, NewType)]
#[oai(from_parameter = false, from_multipart = false, to_header = false)]
pub struct MoveStructTagWrapper(pub MoveStructTag);

impl FromStr for MoveStructTagWrapper {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        Ok(MoveStructTagWrapper(MoveStructTag::from_str(s)?))
    }
}

impl From<MoveStructTagWrapper> for MoveStructTag {
    fn from(value: MoveStructTagWrapper) -> MoveStructTag {
        value.0
    }
}

impl From<MoveStructTag> for MoveStructTagWrapper {
    fn from(value: MoveStructTag) -> MoveStructTagWrapper {
        Self(value)
    }
}

impl fmt::Display for MoveStructTagWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        MoveStructTag::fmt(&self.0, f)
    }
}

impl_poem_parameter!(MoveStructTagWrapper);

// Currently it is not possible to deserialize certain MoveTypes, such as
// generic type params. In those cases, we give up on parsing them as
// MoveTypes and just store the original string representation. This type is
// a painful necessity, we should try to remove it as soon as it becomes
// possible to do so, perhaps as part of removing all the move type conversion
// at the API layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Union)]
pub enum MoveTypeWrapper {
    Parsed(MoveType),
    Raw(String),
}

impl FromStr for MoveTypeWrapper {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, anyhow::Error> {
        match MoveType::from_str(s) {
            Ok(move_type) => Ok(Self::Parsed(move_type)),
            Err(_) => Ok(Self::Raw(s.to_string())),
        }
    }
}

impl From<MoveType> for MoveTypeWrapper {
    fn from(value: MoveType) -> MoveTypeWrapper {
        Self::Parsed(value)
    }
}

impl fmt::Display for MoveTypeWrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Parsed(move_type) => move_type.fmt(f),
            Self::Raw(s) => write!(f, "{}", s),
        }
    }
}

impl From<TypeTag> for MoveTypeWrapper {
    fn from(tag: TypeTag) -> Self {
        Self::Parsed(MoveType::from(tag))
    }
}

impl TryFrom<MoveTypeWrapper> for TypeTag {
    type Error = anyhow::Error;
    fn try_from(move_type_wrapper: MoveTypeWrapper) -> anyhow::Result<Self> {
        match move_type_wrapper {
            MoveTypeWrapper::Parsed(move_type) => Ok(TypeTag::try_from(move_type)?),
            MoveTypeWrapper::Raw(raw) => Err(anyhow!(
                "Could not parse type tag from raw move type: {}",
                raw
            )),
        }
    }
}

impl MoveTypeWrapper {
    pub fn json_type_name(&self) -> Result<String> {
        match self {
            MoveTypeWrapper::Parsed(move_type) => Ok(move_type.json_type_name()),
            MoveTypeWrapper::Raw(raw) => Err(anyhow!(
                "Could not get json type name from raw move type string: {}",
                raw
            )),
        }
    }

    pub fn is_signer(&self) -> bool {
        match self {
            MoveTypeWrapper::Parsed(move_type) => move_type.is_signer(),
            MoveTypeWrapper::Raw(_raw) => false,
        }
    }
}
