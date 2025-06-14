// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Suppose we have the following data structure in a smart contract:
//! ```move
//! struct B {
//!   Map<String, String> mymap;
//! }
//!
//! struct A {
//!   B b;
//!   int my_int;
//! }
//!
//! struct C {
//!   List<int> mylist;
//! }
//!
//! A a;
//! C c;
//! ```
//!
//! and the data belongs to Alice. Then an access to `a.b.mymap` would be translated to an access
//! to an entry in key-value store whose key is `<Alice>/a/b/mymap`. In the same way, the access to
//! `c.mylist` would need to query `<Alice>/c/mylist`.
//!
//! So an account stores its data in a directory structure, for example:
//! ```text
//!   <Alice>/balance:   10
//!   <Alice>/a/b/mymap: {"Bob" => "abcd", "Carol" => "efgh"}
//!   <Alice>/a/myint:   20
//!   <Alice>/c/mylist:  [3, 5, 7, 9]
//! ```
//! If someone needs to query the map above and find out what value associated with "Bob" is,
//! `address` will be set to Alice and `path` will be set to `/a/b/mymap/Bob`.
//!
//! On the other hand, if you want to query only `<Alice>/a/*`, `address` will be set to Alice and
//! `path` will be set to `/a` and use the `get_prefix()` method from statedb

use crate::{
    account_address::AccountAddress,
    state_store::state_key::{inner::StateKeyInner, StateKey},
};
use anyhow::{Error, Result};
use aptos_crypto::hash::HashValue;
use move_core_types::language_storage::{ModuleId, StructTag};
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt, fmt::Formatter};

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct AccessPath {
    pub address: AccountAddress,
    #[serde(with = "serde_bytes")]
    pub path: Vec<u8>,
}

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for AccessPath {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (any::<AccountAddress>(), any::<Path>())
            .prop_map(|(address, path)| AccessPath {
                address,
                path: bcs::to_bytes(&path).unwrap(),
            })
            .boxed()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum Path {
    Code(ModuleId),
    Resource(StructTag),
    ResourceGroup(StructTag),
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Path::Code(module_id) => {
                write!(f, "Code({})", module_id)
            },
            Path::Resource(struct_tag) => {
                write!(f, "Resource({})", struct_tag.to_canonical_string())
            },
            Path::ResourceGroup(struct_tag) => {
                write!(f, "ResourceGroup({})", struct_tag.to_canonical_string())
            },
        }
    }
}

pub enum PathType {
    Code,
    Resource,
    ResourceGroup,
}

impl AccessPath {
    pub fn new(address: AccountAddress, path: Vec<u8>) -> Self {
        AccessPath { address, path }
    }

    pub fn resource_path_vec(tag: StructTag) -> Result<Vec<u8>> {
        let r = bcs::to_bytes(&Path::Resource(tag))?;
        Ok(r)
    }

    /// Convert Accesses into a byte offset which would be used by the storage layer to resolve
    /// where fields are stored.
    pub fn resource_access_path(address: AccountAddress, type_: StructTag) -> Result<AccessPath> {
        Ok(AccessPath {
            address,
            path: AccessPath::resource_path_vec(type_)?,
        })
    }

    pub fn resource_group_path_vec(tag: StructTag) -> Vec<u8> {
        bcs::to_bytes(&Path::ResourceGroup(tag)).expect("Unexpected serialization error")
    }

    /// Convert Accesses into a byte offset which would be used by the storage layer to resolve
    /// where fields are stored.
    pub fn resource_group_access_path(address: AccountAddress, type_: StructTag) -> AccessPath {
        AccessPath {
            address,
            path: AccessPath::resource_group_path_vec(type_),
        }
    }

    pub fn code_path_vec(key: ModuleId) -> Vec<u8> {
        bcs::to_bytes(&Path::Code(key)).expect("Unexpected serialization error")
    }

    pub fn code_access_path(key: ModuleId) -> Self {
        let address = *key.address();
        let path = AccessPath::code_path_vec(key);
        AccessPath { address, path }
    }

    /// Extract the structured resource or module `Path` from `self`
    pub fn get_path(&self) -> Path {
        bcs::from_bytes::<Path>(&self.path).expect("Unexpected serialization error")
    }

    /// Extract a StructTag from `self`. Returns Some if this is a resource access
    /// path and None otherwise
    pub fn get_struct_tag(&self) -> Option<StructTag> {
        match self.get_path() {
            Path::Resource(s) => Some(s),
            Path::ResourceGroup(s) => Some(s),
            Path::Code(_) => None,
        }
    }

    /// Extracts a [ModuleId]. Returns [None] if this is not a module access.
    pub fn try_get_module_id(&self) -> Option<ModuleId> {
        match self.get_path() {
            Path::Code(module_id) => Some(module_id),
            Path::Resource(_) | Path::ResourceGroup(_) => None,
        }
    }

    pub fn is_code(&self) -> bool {
        matches!(self.get_path(), Path::Code(_))
    }

    pub fn is_resource_group(&self) -> bool {
        matches!(self.get_path(), Path::ResourceGroup(_))
    }

    pub fn size(&self) -> usize {
        self.address.as_ref().len() + self.path.len()
    }
}

impl fmt::Debug for AccessPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AccessPath {{ address: 0x{}, path: {:?} }}",
            self.address.short_str_lossless(),
            bcs::from_bytes::<Path>(&self.path)
                .map_or_else(|_| hex::encode(&self.path), |path| format!("{}", path)),
        )
    }
}

impl fmt::Display for AccessPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.path.len() < 1 + HashValue::LENGTH {
            write!(f, "{:?}", self)
        } else {
            write!(f, "AccessPath {{ address: {:x}, ", self.address)?;
            match self.path[0] {
                p if p == PathType::Resource as u8 => write!(f, "type: Resource, ")?,
                p if p == PathType::Code as u8 => write!(f, "type: Module, ")?,
                p if p == PathType::ResourceGroup as u8 => write!(f, "type: ResourceGroup, ")?,
                tag => write!(f, "type: {:?}, ", tag)?,
            };
            write!(
                f,
                "hash: {:?}, ",
                hex::encode(&self.path[1..=HashValue::LENGTH])
            )?;
            write!(
                f,
                "suffix: {:?} }} ",
                String::from_utf8_lossy(&self.path[1 + HashValue::LENGTH..])
            )
        }
    }
}

impl From<&ModuleId> for AccessPath {
    fn from(id: &ModuleId) -> AccessPath {
        AccessPath {
            address: *id.address(),
            path: id.access_vector(),
        }
    }
}

impl TryFrom<StateKey> for AccessPath {
    type Error = Error;

    fn try_from(state_key: StateKey) -> Result<Self> {
        match state_key.inner() {
            StateKeyInner::AccessPath(access_path) => Ok(access_path.clone()),
            _ => anyhow::bail!("Unsupported state key type"),
        }
    }
}

impl TryFrom<&[u8]> for Path {
    type Error = bcs::Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Path>(bytes)
    }
}

impl TryFrom<&Vec<u8>> for Path {
    type Error = bcs::Error;

    fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
        bcs::from_bytes::<Path>(bytes)
    }
}
