// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::non_canonical_partial_ord_impl)]
// FIXME(aldenhu): remove
#![allow(dead_code)]

use crate::{access_path::AccessPath, state_store::table::TableHandle};
use anyhow::Result;
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher, DummyHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use bytes::Bytes;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cmp::Ordering,
    convert::TryInto,
    fmt,
    fmt::{Debug, Formatter},
    hash::Hash,
    sync::Arc,
};
use thiserror::Error;

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

impl fmt::Debug for StateKeyInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StateKeyInner::AccessPath(ap) => {
                write!(f, "{:?}", ap)
            },
            StateKeyInner::TableItem { handle, key } => {
                write!(
                    f,
                    "TableItem {{ handle: {:x}, key: {} }}",
                    handle.0,
                    hex::encode(key),
                )
            },
            StateKeyInner::Raw(bytes) => {
                write!(f, "Raw({})", hex::encode(bytes),)
            },
        }
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

#[repr(u8)]
#[derive(Clone, Debug, FromPrimitive, ToPrimitive)]
pub enum StateKeyTag {
    AccessPath,
    TableItem,
    Raw = 255,
}

impl StateKeyInner {
    fn encode(&self) -> Result<Vec<u8>> {
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

    fn decode(val: &[u8]) -> Result<Self, StateKeyDecodeErr> {
        if val.is_empty() {
            return Err(StateKeyDecodeErr::EmptyInput);
        }
        let tag = val[0];
        let state_key_tag =
            StateKeyTag::from_u8(tag).ok_or(StateKeyDecodeErr::UnknownTag { unknown_tag: tag })?;
        match state_key_tag {
            StateKeyTag::AccessPath => Ok(Self::AccessPath(bcs::from_bytes(&val[1..])?)),
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
                let key = val[1 + HANDLE_SIZE..].to_vec();
                Ok(Self::TableItem { handle, key })
            },
            StateKeyTag::Raw => Ok(Self::Raw(val[1..].to_vec())),
        }
    }
}

#[derive(Debug)]
struct StateKeyInfo {
    pub deserialized: StateKeyInner,
    pub hash_value: HashValue,
    pub serialized: Bytes,
}

impl StateKeyInfo {
    pub fn from_deserialized(deserialized: StateKeyInner) -> Self {
        let hash_value = CryptoHash::hash(&deserialized);
        let serialized = bcs::to_bytes(&deserialized)
            .expect("Failed to serialize StateKeyInner")
            .into();

        Self {
            deserialized,
            hash_value,
            serialized,
        }
    }

    pub fn from_serialized(serialized: Bytes) -> Result<Self> {
        let deserialized = bcs::from_bytes(&serialized)?;
        let hash_value = CryptoHash::hash(&deserialized);
        Ok(Self {
            deserialized,
            hash_value,
            serialized,
        })
    }
}

#[derive(Clone, Debug)]
pub struct StateKey(Arc<StateKeyInfo>);

impl StateKey {
    pub fn size(&self) -> usize {
        match self.inner() {
            StateKeyInner::AccessPath(access_path) => access_path.size(),
            StateKeyInner::TableItem { handle, key } => handle.size() + key.len(),
            StateKeyInner::Raw(bytes) => bytes.len(),
        }
    }

    pub fn inner(&self) -> &StateKeyInner {
        &self.0.deserialized
    }

    pub fn get_shard_id(&self) -> u8 {
        self.0.hash_value.nibble(0)
    }

    pub fn is_aptos_code(&self) -> bool {
        use move_core_types::account_address::AccountAddress;
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

    fn from_deserialized(_deserialized: StateKeyInner) -> Self {
        todo!()
    }

    pub fn table_item(_handle: TableHandle, _key: Vec<u8>) -> Self {
        // FIXME(aldenhu): remove
        todo!()
    }

    pub fn access_path(_access_path: AccessPath) -> Self {
        // FIXME(aldenhu): remove
        todo!()
    }

    pub fn raw(_bytes: Vec<u8>) -> Self {
        // FIXME(aldenhu): remove
        todo!()
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        todo!()
    }

    pub fn decode(_val: &[u8]) -> Result<Self, StateKeyDecodeErr> {
        todo!()
    }
}

impl CryptoHash for StateKey {
    type Hasher = DummyHasher;

    fn hash(&self) -> HashValue {
        self.0.hash_value
    }
}

impl Serialize for StateKey {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // FIXME(aldenhu): write out cased bytes directly; or provide method to access serialized bytes
        todo!()
    }
}

impl<'de> Deserialize<'de> for StateKey {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // FIXME(aldenhu): check cache
        todo!()
    }
}

impl PartialEq for StateKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for StateKey {}

impl Hash for StateKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // FIXME(aldenhu): does it make a difference to hash less bytes?
        state.write(&self.0.hash_value.as_ref()[0..16])
    }
}

impl PartialOrd for StateKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // TODO: consider more efficient PartialOrd && Ord, maybe on another wrapper type, so keys
        //       can be hosted more cheaply in a BTreeSet
        self.0.deserialized.partial_cmp(&other.0.deserialized)
    }
}

impl Ord for StateKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.deserialized.cmp(&other.0.deserialized)
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl proptest::arbitrary::Arbitrary for StateKey {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        use proptest::strategy::Strategy;

        proptest::prelude::any::<StateKeyInner>()
            .prop_map(StateKey::from_deserialized)
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use crate::state_store::state_key::{AccessPath, StateKey};
    use aptos_crypto::hash::CryptoHash;
    use move_core_types::language_storage::ModuleId;

    #[test]
    fn test_access_path_hash() {
        let key = StateKey::access_path(AccessPath::new("0x1002".parse().unwrap(), vec![7, 2, 3]));
        let expected_hash = "0e0960bcabe04c40e8814ecc0e6de415163573243fb5059e9951f5890e9481ef"
            .parse()
            .unwrap();
        assert_eq!(CryptoHash::hash(&key), expected_hash);
    }

    #[test]
    fn test_table_item_hash() {
        let key = StateKey::table_item("0x1002".parse().unwrap(), vec![7, 2, 3]);
        let expected_hash = "6f5550015f7a6036f88b2458f98a7e4800aba09e83f8f294dbf70bff77f224e6"
            .parse()
            .unwrap();
        assert_eq!(CryptoHash::hash(&key), expected_hash);
    }

    #[test]
    fn test_raw_hash() {
        let key = StateKey::raw(vec![1, 2, 3]);
        let expected_hash = "655ab5766bc87318e18d9287f32d318e15535d3db9d21a6e5a2b41a51b535aff"
            .parse()
            .unwrap();
        assert_eq!(CryptoHash::hash(&key), expected_hash);
    }

    #[test]
    fn test_debug() {
        // code
        let key = StateKey::access_path(AccessPath::code_access_path(ModuleId::new(
            "0xcafe".parse().unwrap(),
            "my_module".parse().unwrap(),
        )));
        assert_eq!(
            &format!("{:?}", key),
            "StateKey { inner: AccessPath { address: 0xcafe, path: \"Code(000000000000000000000000000000000000000000000000000000000000cafe::my_module)\" }, hash: OnceCell(Uninit) }"
        );

        // resource
        let key = StateKey::access_path(
            AccessPath::resource_access_path(
                "0xcafe".parse().unwrap(),
                "0x1::account::Account".parse().unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(
            &format!("{:?}", key),
            "StateKey { inner: AccessPath { address: 0xcafe, path: \"Resource(0x1::account::Account)\" }, hash: OnceCell(Uninit) }",
        );

        // table item
        let key = StateKey::table_item("0x123".parse().unwrap(), vec![1]);
        assert_eq!(
            &format!("{:?}", key),
            "StateKey { inner: TableItem { handle: 0000000000000000000000000000000000000000000000000000000000000123, key: 01 }, hash: OnceCell(Uninit) }"
        );

        // raw
        let key = StateKey::raw(vec![1, 2, 3]);
        assert_eq!(
            &format!("{:?}", key),
            "StateKey { inner: Raw(010203), hash: OnceCell(Uninit) }"
        );

        // with hash
        let _hash = CryptoHash::hash(&key);
        assert_eq!(
            &format!("{:?}", key),
            "StateKey { inner: Raw(010203), hash: OnceCell(HashValue(655ab5766bc87318e18d9287f32d318e15535d3db9d21a6e5a2b41a51b535aff)) }"
        );
    }
}
