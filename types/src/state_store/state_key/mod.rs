// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::non_canonical_partial_ord_impl)]

pub mod inner;
pub mod prefix;
#[cfg(test)]
mod tests;

use crate::{
    access_path::AccessPath, on_chain_config::OnChainConfig, state_store::table::TableHandle,
};
use anyhow::Result;
use aptos_crypto::{
    hash::{CryptoHash, DummyHasher},
    HashValue,
};
use derivative::Derivative;
use inner::{StateKeyDecodeErr, StateKeyInner, StateKeyTag};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag},
    move_resource::MoveResource,
};
use num_traits::FromPrimitive;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    convert::TryInto,
    fmt,
    fmt::{Debug, Formatter},
    ops::Deref,
};

#[derive(Clone, Derivative)]
#[derivative(PartialEq, PartialOrd, Hash, Ord)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct StateKey {
    inner: StateKeyInner,
    #[derivative(
        Hash = "ignore",
        Ord = "ignore",
        PartialEq = "ignore",
        PartialOrd = "ignore"
    )]
    #[cfg_attr(any(test, feature = "fuzzing"), proptest(value = "OnceCell::new()"))]
    hash: OnceCell<HashValue>,
}

impl Debug for StateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl StateKey {
    pub fn new(inner: StateKeyInner) -> Self {
        Self {
            inner,
            hash: OnceCell::new(),
        }
    }

    /// Recovers from serialized bytes in physical storage.
    pub fn decode(val: &[u8]) -> Result<StateKey, StateKeyDecodeErr> {
        if val.is_empty() {
            return Err(StateKeyDecodeErr::EmptyInput);
        }
        let tag = val[0];
        let state_key_tag =
            StateKeyTag::from_u8(tag).ok_or(StateKeyDecodeErr::UnknownTag { unknown_tag: tag })?;
        match state_key_tag {
            StateKeyTag::AccessPath => {
                Ok(StateKeyInner::AccessPath(bcs::from_bytes(&val[1..])?).into())
            },
            StateKeyTag::TableItem => {
                const HANDLE_SIZE: usize = std::mem::size_of::<TableHandle>();
                if val.len() < 1 + HANDLE_SIZE {
                    return Err(StateKeyDecodeErr::NotEnoughBytes {
                        tag,
                        num_bytes: val.len(),
                    });
                }
                let handle = bcs::from_bytes(
                    val[1..1 + HANDLE_SIZE]
                        .try_into()
                        .expect("Bytes too short."),
                )?;
                Ok(StateKey::table_item(&handle, &val[1 + HANDLE_SIZE..]))
            },
            StateKeyTag::Raw => Ok(StateKey::raw(&val[1..])),
        }
    }

    pub fn size(&self) -> usize {
        match &self.inner {
            StateKeyInner::AccessPath(access_path) => access_path.size(),
            StateKeyInner::TableItem { handle, key } => handle.size() + key.len(),
            StateKeyInner::Raw(bytes) => bytes.len(),
        }
    }

    fn access_path(access_path: AccessPath) -> Self {
        Self::new(StateKeyInner::AccessPath(access_path))
    }

    pub fn resource(address: &AccountAddress, struct_tag: &StructTag) -> Result<Self> {
        Ok(Self::access_path(AccessPath::resource_access_path(
            *address,
            struct_tag.to_owned(),
        )?))
    }

    pub fn resource_typed<T: MoveResource>(address: &AccountAddress) -> Result<Self> {
        Self::resource(address, &T::struct_tag())
    }

    pub fn resource_group(address: &AccountAddress, struct_tag: &StructTag) -> Self {
        Self::access_path(AccessPath::resource_group_access_path(
            *address,
            struct_tag.to_owned(),
        ))
    }

    pub fn module(address: &AccountAddress, name: &IdentStr) -> Self {
        Self::access_path(AccessPath::code_access_path(ModuleId::new(
            *address,
            name.to_owned(),
        )))
    }

    pub fn module_id(module_id: &ModuleId) -> Self {
        Self::module(module_id.address(), module_id.name())
    }

    pub fn on_chain_config<T: OnChainConfig>() -> Result<Self> {
        Self::resource(T::address(), &T::struct_tag())
    }

    pub fn table_item(handle: &TableHandle, key: &[u8]) -> Self {
        Self::new(StateKeyInner::TableItem {
            handle: *handle,
            key: key.to_vec(),
        })
    }

    pub fn raw(raw_key: &[u8]) -> Self {
        Self::new(StateKeyInner::Raw(raw_key.to_vec()))
    }

    pub fn inner(&self) -> &StateKeyInner {
        &self.inner
    }

    pub fn get_shard_id(&self) -> u8 {
        CryptoHash::hash(self).nibble(0)
    }

    pub fn is_aptos_code(&self) -> bool {
        match self.inner() {
            StateKeyInner::AccessPath(access_path) => {
                access_path.is_code()
                    && (access_path.address == AccountAddress::ONE
                        || access_path.address == AccountAddress::THREE
                        || access_path.address == AccountAddress::FOUR)
            },
            _ => false,
        }
    }
}

impl CryptoHash for StateKey {
    type Hasher = DummyHasher;

    fn hash(&self) -> HashValue {
        *self.hash.get_or_init(|| CryptoHash::hash(&self.inner))
    }
}

impl Serialize for StateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = StateKeyInner::deserialize(deserializer)?;
        Ok(Self::new(inner))
    }
}

impl Deref for StateKey {
    type Target = StateKeyInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Eq for StateKey {}

impl From<StateKeyInner> for StateKey {
    fn from(inner: StateKeyInner) -> Self {
        StateKey::new(inner)
    }
}
