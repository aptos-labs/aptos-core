// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::non_canonical_partial_ord_impl)]

use crate::{
    access_path::AccessPath, on_chain_config::OnChainConfig, state_store::table::TableHandle,
};
use anyhow::Result;
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher, DummyHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use derivative::Derivative;
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag},
    move_resource::MoveResource,
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    convert::TryInto,
    fmt,
    fmt::{Debug, Formatter},
    hash::Hash,
    ops::Deref,
};
use thiserror::Error;

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

#[derive(Clone, CryptoHasher, Eq, PartialEq, Serialize, Deserialize, Ord, PartialOrd, Hash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[serde(rename = "StateKey")]
pub enum StateKeyInner {
    AccessPath(AccessPath),
    TableItem {
        handle: TableHandle,
        #[serde(with = "serde_bytes")]
        key: Vec<u8>,
    },
    // Only used for testing
    #[serde(with = "serde_bytes")]
    Raw(Vec<u8>),
}

impl Debug for StateKeyInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StateKeyInner::AccessPath(ap) => {
                write!(f, "StateKey::{:?}", ap)
            },
            StateKeyInner::TableItem { handle, key } => {
                write!(
                    f,
                    "StateKey::TableItem {{ handle: {:x}, key: {} }}",
                    handle.0,
                    hex::encode(key),
                )
            },
            StateKeyInner::Raw(bytes) => {
                write!(f, "StateKey::Raw({})", hex::encode(bytes),)
            },
        }
    }
}

impl Debug for StateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[repr(u8)]
#[derive(Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum StateKeyTag {
    AccessPath,
    TableItem,
    Raw = 255,
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

impl StateKeyInner {
    /// Serializes to bytes for physical storage.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = vec![];

        let (prefix, raw_key) = match self {
            StateKeyInner::AccessPath(access_path) => {
                (StateKeyTag::AccessPath, bcs::to_bytes(access_path)?)
            },
            StateKeyInner::TableItem { handle, key } => {
                let mut bytes = bcs::to_bytes(&handle)?;
                bytes.extend(key);
                (StateKeyTag::TableItem, bytes)
            },
            StateKeyInner::Raw(raw_bytes) => (StateKeyTag::Raw, raw_bytes.to_vec()),
        };
        out.push(prefix as u8);
        out.extend(raw_key);
        Ok(out)
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

impl CryptoHash for StateKeyInner {
    type Hasher = StateKeyInnerHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(
            self.encode()
                .expect("Failed to serialize the state key")
                .as_ref(),
        );
        state.finish()
    }
}

/// Error thrown when a [`StateKey`] fails to be deserialized out of a byte sequence stored in physical
/// storage, via [`StateKey::decode`].
#[derive(Debug, Error)]
pub enum StateKeyDecodeErr {
    /// Input is empty.
    #[error("Missing tag due to empty input")]
    EmptyInput,

    /// The first byte of the input is not a known tag representing one of the variants.
    #[error("lead tag byte is unknown: {}", unknown_tag)]
    UnknownTag { unknown_tag: u8 },

    #[error("Not enough bytes: tag: {}, num bytes: {}", tag, num_bytes)]
    NotEnoughBytes { tag: u8, num_bytes: usize },

    #[error(transparent)]
    BcsError(#[from] bcs::Error),
}

#[cfg(test)]
mod tests {
    use crate::{
        account_config::{AccountResource, ObjectGroupResource},
        state_store::state_key::StateKey,
    };
    use aptos_crypto::hash::CryptoHash;
    use move_core_types::{
        account_address::AccountAddress, ident_str, move_resource::MoveStructType,
    };

    fn assert_crypto_hash(key: &StateKey, expected_hash: &str) {
        let expected_hash = expected_hash.parse().unwrap();
        assert_eq!(CryptoHash::hash(key), expected_hash);
    }

    #[test]
    fn test_resource_hash() {
        assert_crypto_hash(
            &StateKey::resource_typed::<AccountResource>(&AccountAddress::TWO).unwrap(),
            "8f9ab5d5e3c9f5b885fcceea388fecd16bdb490da08aac9d4f026ddc66733def",
        );
    }

    #[test]
    fn test_resource_group_hash() {
        assert_crypto_hash(
            &StateKey::resource_group(&AccountAddress::TWO, &ObjectGroupResource::struct_tag()),
            "87973d52189ac6a25ea543214305c4c8fb3bc2ceea8c34600361b03527578133",
        );
    }

    #[test]
    fn test_module_hash() {
        assert_crypto_hash(
            &StateKey::module(&AccountAddress::TWO, ident_str!("mymodule")),
            "83d33b345c5e4b25d8f4dfe2b98b492024313b3b6e4febea6bfa844dbd850200",
        );
    }

    #[test]
    fn test_table_item_hash() {
        assert_crypto_hash(
            &StateKey::table_item(&"0x1002".parse().unwrap(), &[7, 2, 3]),
            "6f5550015f7a6036f88b2458f98a7e4800aba09e83f8f294dbf70bff77f224e6",
        );
    }

    #[test]
    fn test_raw_hash() {
        assert_crypto_hash(
            &StateKey::raw(&[1, 2, 3]),
            "655ab5766bc87318e18d9287f32d318e15535d3db9d21a6e5a2b41a51b535aff",
        )
    }

    #[test]
    fn test_debug() {
        // code
        let key = StateKey::module(&AccountAddress::ONE, ident_str!("account"));
        assert_eq!(
            &format!("{:?}", key),
            "StateKey::AccessPath { address: 0x1, path: \"Code(0000000000000000000000000000000000000000000000000000000000000001::account)\" }",
        );

        // resource
        let key = StateKey::resource_typed::<AccountResource>(&AccountAddress::FOUR).unwrap();
        assert_eq!(
            &format!("{:?}", key),
            "StateKey::AccessPath { address: 0x4, path: \"Resource(0x1::account::Account)\" }",
        );

        // resource group
        let key =
            StateKey::resource_group(&AccountAddress::THREE, &ObjectGroupResource::struct_tag());
        assert_eq!(
            &format!("{:?}", key),
            "StateKey::AccessPath { address: 0x3, path: \"ResourceGroup(0x1::object::ObjectGroup)\" }",
        );

        // table item
        let key = StateKey::table_item(&"0x123".parse().unwrap(), &[1]);
        assert_eq!(
            &format!("{:?}", key),
            "StateKey::TableItem { handle: 0000000000000000000000000000000000000000000000000000000000000123, key: 01 }"
        );

        // raw
        let key = StateKey::raw(&[1, 2, 3]);
        assert_eq!(&format!("{:?}", key), "StateKey::Raw(010203)",);
    }
}
