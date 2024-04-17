// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::non_canonical_partial_ord_impl)]

use crate::{
    access_path,
    access_path::AccessPath,
    on_chain_config::OnChainConfig,
    state_store::{
        // metrics::{STATE_KEY_COUNTERS, STATE_KEY_TIMER},
        table::TableHandle,
    },
};
use anyhow::Result;
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher, DummyHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_infallible::RwLock;
use bytes::{BufMut, Bytes, BytesMut};
// use aptos_metrics_core::{IntCounterHelper, TimerHelper};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag},
    move_resource::MoveResource,
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    borrow::Borrow,
    cmp::Ordering,
    collections::{hash_map, HashMap},
    convert::TryInto,
    fmt,
    fmt::{Debug, Formatter},
    hash::Hash,
    io::Write,
    ops::Deref,
    sync::{Arc, Weak},
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

impl Debug for StateKeyInner {
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
    fn encode(&self) -> Result<Bytes> {
        let mut writer = BytesMut::new().writer();

        match self {
            StateKeyInner::AccessPath(access_path) => {
                writer.write_all(&[StateKeyTag::AccessPath as u8])?;
                bcs::serialize_into(&mut writer, access_path)?;
            },
            StateKeyInner::TableItem { handle, key } => {
                writer.write_all(&[StateKeyTag::TableItem as u8])?;
                bcs::serialize_into(&mut writer, &handle)?;
                bcs::serialize_into(&mut writer, key)?;
            },
            StateKeyInner::Raw(raw_bytes) => {
                writer.write_all(&[StateKeyTag::TableItem as u8])?;
                writer.write_all(raw_bytes)?;
            },
        };

        Ok(writer.into_inner().into())
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
struct EntryInner {
    pub deserialized: StateKeyInner,
    // pub serialized: Bytes,
    pub encoded: Bytes,
    pub hash_value: HashValue,
}

/// n.b. Wrapping it so EntryInner is constructed outside the lock while Entry is only constructed
///      and dropped if it's ever added to the registry.
#[derive(Debug)]
struct Entry(EntryInner);

impl Deref for Entry {
    type Target = EntryInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl EntryInner {
    pub fn from_deserialized(deserialized: StateKeyInner) -> Self {
        // FIXME(aldenhu): deal with error
        let encoded = deserialized
            .encode()
            .expect("Failed to encode StateKeyInner.");

        let mut state = StateKeyInnerHasher::default();
        // FIXME(aldenhu): check error processing
        state.update(&encoded);
        let hash_value = state.finish();

        Self {
            deserialized,
            encoded,
            hash_value,
        }
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        match &self.deserialized {
            StateKeyInner::AccessPath(AccessPath { address, path }) => {
                use access_path::Path;

                // TODO(aldenhu): maybe hold reference to the map(s)?
                // TODO(aldenhu): maybe let Inner carry the deserialized Path?
                match &bcs::from_bytes::<Path>(path).expect("Failed to deserialize Path.") {
                    Path::Code(module_id) => GLOBAL_REGISTRY
                        .module
                        .lock_and_remove(&module_id.address, &module_id.name),
                    Path::Resource(struct_tag) => GLOBAL_REGISTRY
                        .resource
                        .lock_and_remove(struct_tag, address),
                    Path::ResourceGroup(struct_tag) => GLOBAL_REGISTRY
                        .resource_group
                        .lock_and_remove(struct_tag, address),
                }
            },
            StateKeyInner::TableItem { handle, key } => {
                GLOBAL_REGISTRY.table_item.lock_and_remove(handle, key)
            },
            StateKeyInner::Raw(bytes) => GLOBAL_REGISTRY.raw.lock_and_remove(bytes, &()),
        }
    }
}

struct TwoLevelRegistry<Key1, Key2> {
    // FIXME(aldenhu): remove
    #[allow(dead_code)]
    key_type: &'static str,
    inner: RwLock<HashMap<Key1, HashMap<Key2, Weak<Entry>>>>,
}

impl<Key1, Key2> TwoLevelRegistry<Key1, Key2>
where
    Key1: Clone + Eq + Hash,
    Key2: Clone + Eq + Hash,
{
    fn new_empty(key_type: &'static str) -> Self {
        Self {
            key_type,
            inner: RwLock::new(HashMap::new()),
        }
    }

    fn try_get<Q1, Q2>(&self, key1: &Q1, key2: &Q2) -> Option<Arc<Entry>>
    where
        Key1: Borrow<Q1>,
        Key2: Borrow<Q2>,
        Q1: Eq + Hash + ?Sized,
        Q2: Eq + Hash + ?Sized,
    {
        let locked = match self.inner.inner().try_read() {
            Ok(locked) => locked,
            Err(..) => {
                // blocked by a write lock
                // STATE_KEY_COUNTERS.inc_with(&[self.key_type, "read_blocked_by_write"]);
                // wait for write lock to release
                self.inner.read()
            },
        };

        locked
            .get(key1)
            .and_then(|m| m.get(key2))
            .and_then(|weak| weak.upgrade())
    }

    fn lock_and_get_or_add<Q1, Q2>(
        &self,
        key1: &Q1,
        key2: &Q2,
        maybe_add: StateKeyInner,
    ) -> Arc<Entry>
    where
        Key1: Borrow<Q1>,
        Key2: Borrow<Q2>,
        Q1: Eq + Hash + ToOwned<Owned = Key1> + ?Sized,
        Q2: Eq + Hash + ToOwned<Owned = Key2> + ?Sized,
    {
        // let _timer = STATE_KEY_TIMER.timer_with(&[self.key_type, "lock_and_get_or_add"]);

        // construct the inner entry outside the lock
        let maybe_add = EntryInner::from_deserialized(maybe_add);

        const MAX_TRIES: usize = 100;

        for _ in 0..MAX_TRIES {
            match self
                .inner
                .write()
                .entry(key1.to_owned())
                .or_default()
                .entry(key2.to_owned())
            {
                hash_map::Entry::Occupied(occupied) => {
                    if let Some(entry) = occupied.get().upgrade() {
                        // some other thread has added it
                        // STATE_KEY_COUNTERS.inc_with(&[self.key_type, "entry_create_collision"]);
                        return entry;
                    } else {
                        // the key is being dropped, release lock and retry
                        // STATE_KEY_COUNTERS
                        //     .inc_with(&[self.key_type, "entry_create_while_dropping"]);
                        continue;
                    }
                },
                hash_map::Entry::Vacant(vacant) => {
                    // STATE_KEY_COUNTERS.inc_with(&[self.key_type, "entry_create"]);

                    let entry = Arc::new(Entry(maybe_add));
                    vacant.insert(Arc::downgrade(&entry));
                    return entry;
                },
            }
        }
        unreachable!("Looks like deadlock");
    }

    fn lock_and_remove(&self, key1: &Key1, key2: &Key2) {
        match self.inner.write().entry(key1.to_owned()) {
            hash_map::Entry::Occupied(mut occupied) => {
                match occupied.get_mut().remove(key2) {
                    Some(..) => {
                        // STATE_KEY_COUNTERS.inc_with(&[self.key_type, "entry_remove"]);
                    },
                    None => {
                        unreachable!("Entry missing in registry when dropping.")
                    },
                }
                if occupied.get().is_empty() {
                    occupied.remove();
                }
            },
            hash_map::Entry::Vacant(_) => {
                // This should not happen
                unreachable!("level 1 map must exist when an entry is supposed to be in it.");
            },
        }
    }
}

static GLOBAL_REGISTRY: Lazy<StateKeyRegistry> = Lazy::new(StateKeyRegistry::new_empty);

pub struct StateKeyRegistry {
    // FIXME(aldenhu): reverse dimensions to save memory?
    resource: TwoLevelRegistry<StructTag, AccountAddress>,
    resource_group: TwoLevelRegistry<StructTag, AccountAddress>,
    module: TwoLevelRegistry<AccountAddress, Identifier>,
    table_item: TwoLevelRegistry<TableHandle, Vec<u8>>,
    raw: TwoLevelRegistry<Vec<u8>, ()>, // for tests only
}

impl StateKeyRegistry {
    fn new_empty() -> Self {
        Self {
            resource: TwoLevelRegistry::new_empty("resource"),
            resource_group: TwoLevelRegistry::new_empty("resource_group"),
            module: TwoLevelRegistry::new_empty("module"),
            table_item: TwoLevelRegistry::new_empty("table_item"),
            raw: TwoLevelRegistry::new_empty("raw"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StateKey(Arc<Entry>);

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

    fn from_deserialized(deserialized: StateKeyInner) -> Self {
        use access_path::Path;

        match deserialized.clone() {
            StateKeyInner::AccessPath(AccessPath { address, path }) => {
                match bcs::from_bytes::<Path>(&path).expect("Failed to parse AccessPath") {
                    Path::Code(module_id) => {
                        Self::module_(module_id.address(), module_id.name(), Some(deserialized))
                    },
                    Path::Resource(struct_tag) => {
                        Self::resource_(&address, &struct_tag, Some(deserialized))
                    },
                    Path::ResourceGroup(struct_tag) => {
                        Self::resource_group_(&address, &struct_tag, Some(deserialized))
                    },
                }
            },
            StateKeyInner::TableItem { handle, key } => {
                Self::table_item_(&handle, &key, Some(deserialized))
            },
            StateKeyInner::Raw(bytes) => Self::raw_(&bytes, Some(deserialized)),
        }
    }

    fn resource_(
        address: &AccountAddress,
        struct_tag: &StructTag,
        deserialized: Option<StateKeyInner>,
    ) -> Self {
        if let Some(entry) = GLOBAL_REGISTRY.resource.try_get(struct_tag, address) {
            return Self(entry);
        }

        Self(GLOBAL_REGISTRY.resource.lock_and_get_or_add(
            struct_tag,
            address,
            deserialized.unwrap_or_else(|| {
                StateKeyInner::AccessPath(
                    AccessPath::resource_access_path(*address, struct_tag.clone())
                        .expect("Failed to create access path"),
                )
            }),
        ))
    }

    pub fn resource(address: &AccountAddress, struct_tag: &StructTag) -> Self {
        Self::resource_(address, struct_tag, None)
    }

    pub fn resource_typed<T: MoveResource>(address: &AccountAddress) -> Self {
        Self::resource(address, &T::struct_tag())
    }

    pub fn on_chain_config<T: OnChainConfig>() -> Self {
        Self::resource(T::address(), &T::struct_tag())
    }

    pub fn resource_group_(
        address: &AccountAddress,
        struct_tag: &StructTag,
        deserialized: Option<StateKeyInner>,
    ) -> Self {
        if let Some(entry) = GLOBAL_REGISTRY.resource_group.try_get(struct_tag, address) {
            return Self(entry);
        }

        Self(GLOBAL_REGISTRY.resource_group.lock_and_get_or_add(
            struct_tag,
            address,
            deserialized.unwrap_or_else(|| {
                StateKeyInner::AccessPath(AccessPath::resource_group_access_path(
                    *address,
                    struct_tag.clone(),
                ))
            }),
        ))
    }

    pub fn resource_group(address: &AccountAddress, struct_tag: &StructTag) -> Self {
        Self::resource_group_(address, struct_tag, None)
    }

    pub fn module_(
        address: &AccountAddress,
        name: &IdentStr,
        deserialized: Option<StateKeyInner>,
    ) -> Self {
        if let Some(entry) = GLOBAL_REGISTRY.module.try_get(address, name) {
            return Self(entry);
        }

        Self(GLOBAL_REGISTRY.module.lock_and_get_or_add(
            address,
            name,
            deserialized.unwrap_or_else(|| {
                StateKeyInner::AccessPath(AccessPath::code_access_path(ModuleId::new(
                    *address,
                    name.to_owned(),
                )))
            }),
        ))
    }

    pub fn module(address: &AccountAddress, name: &IdentStr) -> Self {
        Self::module_(address, name, None)
    }

    pub fn module_id(module_id: &ModuleId) -> Self {
        Self::module(&module_id.address, &module_id.name)
    }

    pub fn table_item_(
        handle: &TableHandle,
        key: &[u8],
        deserialized: Option<StateKeyInner>,
    ) -> Self {
        if let Some(entry) = GLOBAL_REGISTRY.table_item.try_get(handle, key) {
            return Self(entry);
        }

        Self(GLOBAL_REGISTRY.table_item.lock_and_get_or_add(
            handle,
            key,
            deserialized.unwrap_or_else(|| StateKeyInner::TableItem {
                handle: *handle,
                key: key.to_vec(),
            }),
        ))
    }

    pub fn table_item(handle: &TableHandle, key: &[u8]) -> Self {
        Self::table_item_(handle, key, None)
    }

    pub fn raw_(bytes: &[u8], deserialized: Option<StateKeyInner>) -> Self {
        if let Some(entry) = GLOBAL_REGISTRY.raw.try_get(bytes, &()) {
            return Self(entry);
        }

        Self(GLOBAL_REGISTRY.raw.lock_and_get_or_add(
            bytes,
            &(),
            deserialized.unwrap_or_else(|| StateKeyInner::Raw(bytes.to_vec())),
        ))
    }

    pub fn raw(bytes: &[u8]) -> Self {
        Self::raw_(bytes, None)
    }

    pub fn encode(&self) -> Result<Bytes> {
        Ok(self.0.encoded.clone())
    }

    pub fn decode(_val: &[u8]) -> Result<Self, StateKeyDecodeErr> {
        // FIXME(aldenhu): maybe check cache?
        let inner = StateKeyInner::decode(_val)?;
        Ok(Self::from_deserialized(inner))
    }
}

impl CryptoHash for StateKey {
    type Hasher = DummyHasher;

    fn hash(&self) -> HashValue {
        self.0.hash_value
    }
}

impl Serialize for StateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // FIXME(aldenhu): write out cased bytes directly; or provide method to access serialized bytes
        self.inner().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // FIXME(aldenhu): check cache
        let inner = StateKeyInner::deserialize(deserializer)?;
        Ok(Self::from_deserialized(inner))
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
    use crate::state_store::state_key::StateKey;
    use aptos_crypto::hash::CryptoHash;
    use move_core_types::language_storage::ModuleId;

    /* FIXME(aldenhu): re-implement
    #[test]
    fn test_access_path_hash() {
        let key = StateKey::access_path(AccessPath::new("0x1002".parse().unwrap(), vec![7, 2, 3]));
        let expected_hash = "0e0960bcabe04c40e8814ecc0e6de415163573243fb5059e9951f5890e9481ef"
            .parse()
            .unwrap();
        assert_eq!(CryptoHash::hash(&key), expected_hash);
    }
     */

    #[test]
    fn test_table_item_hash() {
        let key = StateKey::table_item(&"0x1002".parse().unwrap(), &[7, 2, 3]);
        let expected_hash = "6f5550015f7a6036f88b2458f98a7e4800aba09e83f8f294dbf70bff77f224e6"
            .parse()
            .unwrap();
        assert_eq!(CryptoHash::hash(&key), expected_hash);
    }

    #[test]
    fn test_raw_hash() {
        let key = StateKey::raw(&[1, 2, 3]);
        let expected_hash = "655ab5766bc87318e18d9287f32d318e15535d3db9d21a6e5a2b41a51b535aff"
            .parse()
            .unwrap();
        assert_eq!(CryptoHash::hash(&key), expected_hash);
    }

    #[test]
    fn test_debug() {
        // code
        let key = StateKey::module_id(&ModuleId::new(
            "0xcafe".parse().unwrap(),
            "my_module".parse().unwrap(),
        ));
        assert_eq!(
            &format!("{:?}", key),
            "StateKey { inner: AccessPath { address: 0xcafe, path: \"Code(000000000000000000000000000000000000000000000000000000000000cafe::my_module)\" }, hash: OnceCell(Uninit) }"
        );

        // resource
        let key = StateKey::resource(
            &"0xcafe".parse().unwrap(),
            &"0x1::account::Account".parse().unwrap(),
        );
        assert_eq!(
            &format!("{:?}", key),
            "StateKey { inner: AccessPath { address: 0xcafe, path: \"Resource(0x1::account::Account)\" }, hash: OnceCell(Uninit) }",
        );

        // table item
        let key = StateKey::table_item(&"0x123".parse().unwrap(), &[1]);
        assert_eq!(
            &format!("{:?}", key),
            "StateKey { inner: TableItem { handle: 0000000000000000000000000000000000000000000000000000000000000123, key: 01 }, hash: OnceCell(Uninit) }"
        );

        // raw
        let key = StateKey::raw(&[1, 2, 3]);
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
