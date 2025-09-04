// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path::AccessPath,
    state_store::{
        state_key::inner::{StateKeyInner, StateKeyInnerHasher},
        table::TableHandle,
    },
};
use anyhow::Result;
use velor_crypto::{hash::CryptoHasher, HashValue};
use velor_infallible::RwLock;
use bytes::Bytes;
use hashbrown::HashMap;
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::StructTag,
};
use once_cell::sync::Lazy;
use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

#[derive(Debug)]
pub struct Entry {
    pub deserialized: StateKeyInner,
    pub encoded: Bytes,
    pub hash_value: HashValue,
}

impl Entry {
    fn new(deserialized: StateKeyInner, encoded: Bytes, hash_value: HashValue) -> Self {
        Entry {
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
                use crate::access_path::Path;

                match &bcs::from_bytes::<Path>(path).expect("Failed to deserialize Path.") {
                    Path::Code(module_id) => REGISTRY
                        .module(address, &module_id.name)
                        .maybe_remove(&module_id.address, &module_id.name),
                    Path::Resource(struct_tag) => REGISTRY
                        .resource(struct_tag, address)
                        .maybe_remove(struct_tag, address),
                    Path::ResourceGroup(struct_tag) => REGISTRY
                        .resource_group(struct_tag, address)
                        .maybe_remove(struct_tag, address),
                }
            },
            StateKeyInner::TableItem { handle, key } => {
                REGISTRY.table_item(handle, key).maybe_remove(handle, key)
            },
            StateKeyInner::Raw(bytes) => REGISTRY.raw(bytes).maybe_remove(bytes, &()),
        }
    }
}

pub(crate) struct TwoKeyRegistry<Key1, Key2> {
    inner: RwLock<HashMap<Key1, HashMap<Key2, Weak<Entry>>>>,
}

impl<Key1, Key2> TwoKeyRegistry<Key1, Key2>
where
    Key1: Clone + Eq + Hash,
    Key2: Clone + Eq + Hash,
{
    fn read_lock_try_get<Ref1, Ref2>(&self, key1: &Ref1, key2: &Ref2) -> Option<Arc<Entry>>
    where
        Key1: Borrow<Ref1>,
        Key2: Borrow<Ref2>,
        Ref1: Eq + Hash + ?Sized,
        Ref2: Eq + Hash + ?Sized,
    {
        self.inner
            .read()
            .get(key1)
            .and_then(|m| m.get(key2))
            .and_then(|weak| weak.upgrade())
    }

    fn insert_key2(map2: &mut HashMap<Key2, Weak<Entry>>, key2: Key2, entry: Entry) -> Arc<Entry> {
        let entry = Arc::new(entry);
        map2.insert(key2, Arc::downgrade(&entry));
        entry
    }

    fn write_lock_get_or_add<Ref1, Ref2, Gen>(
        &self,
        key1: &Ref1,
        key2: &Ref2,
        inner_gen: Gen,
    ) -> Result<Arc<Entry>>
    where
        Key1: Borrow<Ref1>,
        Key2: Borrow<Ref2>,
        Ref1: Eq + Hash + ToOwned<Owned = Key1> + ?Sized,
        Ref2: Eq + Hash + ToOwned<Owned = Key2> + ?Sized,
        Gen: FnOnce() -> Result<StateKeyInner>,
    {
        // generate the entry content outside the lock
        // n.b. construct Entry only when decided to insert to registry, to save on drop
        let deserialized = inner_gen()?;
        let encoded = deserialized.encode().expect("Failed to encode StateKey.");
        let hash_value = {
            let mut state = StateKeyInnerHasher::default();
            state.update(&encoded);
            state.finish()
        };

        let mut locked = self.inner.write();

        Ok(match locked.get_mut(key1) {
            None => {
                let mut map2 = locked.entry(key1.to_owned()).insert(HashMap::new());
                let entry = Entry::new(deserialized, encoded, hash_value);
                Self::insert_key2(map2.get_mut(), key2.to_owned(), entry)
            },
            Some(map2) => match map2.get(key2) {
                None => {
                    let entry = Entry::new(deserialized, encoded, hash_value);
                    Self::insert_key2(map2, key2.to_owned(), entry)
                },
                Some(weak) => match weak.upgrade() {
                    Some(entry) => {
                        // some other thread has added it
                        entry
                    },
                    None => {
                        // previous version of this key is being dropped.
                        let entry = Entry::new(deserialized, encoded, hash_value);
                        Self::insert_key2(map2, key2.to_owned(), entry)
                    },
                },
            },
        })
    }

    fn maybe_remove(&self, key1: &Key1, key2: &Key2) {
        let mut locked = self.inner.write();
        if let Some(map2) = locked.get_mut(key1) {
            if let Some(entry) = map2.get(key2) {
                if entry.strong_count() == 0 {
                    map2.remove(key2);
                    if map2.is_empty() {
                        locked.remove(key1);
                    }
                }
            }
        }
    }

    pub fn get_or_add<Ref1, Ref2, Gen>(
        &self,
        key1: &Ref1,
        key2: &Ref2,
        inner_gen: Gen,
    ) -> Result<Arc<Entry>>
    where
        Key1: Borrow<Ref1>,
        Key2: Borrow<Ref2>,
        Ref1: Eq + Hash + ToOwned<Owned = Key1> + ?Sized,
        Ref2: Eq + Hash + ToOwned<Owned = Key2> + ?Sized,
        Gen: FnOnce() -> Result<StateKeyInner>,
    {
        if let Some(entry) = self.read_lock_try_get(key1, key2) {
            return Ok(entry);
        }

        self.write_lock_get_or_add(key1, key2, inner_gen)
    }
}

impl<Key1, Key2> Default for TwoKeyRegistry<Key1, Key2> {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

pub static REGISTRY: Lazy<StateKeyRegistry> = Lazy::new(StateKeyRegistry::default);

const NUM_RESOURCE_SHARDS: usize = 8;
const NUM_RESOURCE_GROUP_SHARDS: usize = 8;
const NUM_MODULE_SHARDS: usize = 8;
const NUM_TABLE_ITEM_SHARDS: usize = 8;
const NUM_RAW_SHARDS: usize = 4;

#[derive(Default)]
pub struct StateKeyRegistry {
    resource_shards: [TwoKeyRegistry<StructTag, AccountAddress>; NUM_RESOURCE_SHARDS],
    resource_group_shards: [TwoKeyRegistry<StructTag, AccountAddress>; NUM_RESOURCE_GROUP_SHARDS],
    module_shards: [TwoKeyRegistry<AccountAddress, Identifier>; NUM_MODULE_SHARDS],
    table_item_shards: [TwoKeyRegistry<TableHandle, Vec<u8>>; NUM_TABLE_ITEM_SHARDS],
    raw_shards: [TwoKeyRegistry<Vec<u8>, ()>; NUM_RAW_SHARDS], // for tests only
}

impl StateKeyRegistry {
    pub fn hash_address_and_name(address: &AccountAddress, name: &[u8]) -> usize {
        let mut hasher = fxhash::FxHasher::default();
        hasher.write_u8(address.as_ref()[AccountAddress::LENGTH - 1]);
        if !name.is_empty() {
            hasher.write_u8(name[0]);
            hasher.write_u8(name[name.len() - 1]);
        }
        hasher.finish() as usize
    }

    pub(crate) fn resource(
        &self,
        struct_tag: &StructTag,
        address: &AccountAddress,
    ) -> &TwoKeyRegistry<StructTag, AccountAddress> {
        &self.resource_shards
            [Self::hash_address_and_name(address, struct_tag.name.as_bytes()) % NUM_RESOURCE_SHARDS]
    }

    pub(crate) fn resource_group(
        &self,
        struct_tag: &StructTag,
        address: &AccountAddress,
    ) -> &TwoKeyRegistry<StructTag, AccountAddress> {
        &self.resource_group_shards[Self::hash_address_and_name(
            address,
            struct_tag.name.as_bytes(),
        ) % NUM_RESOURCE_GROUP_SHARDS]
    }

    pub(crate) fn module(
        &self,
        address: &AccountAddress,
        name: &IdentStr,
    ) -> &TwoKeyRegistry<AccountAddress, Identifier> {
        &self.module_shards
            [Self::hash_address_and_name(address, name.as_bytes()) % NUM_MODULE_SHARDS]
    }

    pub(crate) fn table_item(
        &self,
        handle: &TableHandle,
        key: &[u8],
    ) -> &TwoKeyRegistry<TableHandle, Vec<u8>> {
        &self.table_item_shards[Self::hash_address_and_name(&handle.0, key) % NUM_MODULE_SHARDS]
    }

    pub(crate) fn raw(&self, bytes: &[u8]) -> &TwoKeyRegistry<Vec<u8>, ()> {
        &self.raw_shards[Self::hash_address_and_name(&AccountAddress::ONE, bytes) % NUM_RAW_SHARDS]
    }
}
