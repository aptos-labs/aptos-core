// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Error;
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{access_path::AccessPath, state_store::state_key::StateKey};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::StructTag,
    metadata::Metadata,
    resolver::ResourceResolver,
    value::{LayoutTag, MoveStructLayout, MoveTypeLayout},
};
use move_vm_types::{
    value_exchange::{
        deserialize_and_exchange, AsIdentifier, ExchangeResult, IdentifierBuilder, ValueExchange,
    },
    values::{Struct, Value},
};
use std::{cell::RefCell, collections::BTreeMap, str::FromStr};

/// Models a storage backend which only stores bytes of data. Storage can be
/// resolved like anything else, to get a resource, but it does not do anything
/// special.
#[derive(Debug, Default)]
struct Storage {
    inner: BTreeMap<StateKey, Vec<u8>>,
}

impl ResourceResolver for Storage {
    fn get_resource_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
        _metadata: &[Metadata],
        _layout: Option<&MoveTypeLayout>,
    ) -> anyhow::Result<(Option<Vec<u8>>, usize), anyhow::Error> {
        let ap = AccessPath::resource_access_path(*address, typ.clone())
            .expect("Access path for resource have to be valid");
        let state_key = StateKey::access_path(ap);
        Ok(match self.inner.get(&state_key) {
            Some(blob) => (Some(blob.clone()), blob.len()),
            None => (None, 0),
        })
    }
}

impl TableResolver for Storage {
    fn resolve_table_entry_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        _layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let state_key = StateKey::table_item((*handle).into(), key.to_vec());
        Ok(self.inner.get(&state_key).cloned())
    }
}

/// Models a cache on top of storage. In addition to cached values, it also
/// stores a lifting map of extracted aggregator values.
#[derive(Debug, Default)]
struct Cache {
    liftings: RefCell<BTreeMap<u64, Value>>,
    inner: BTreeMap<StateKey, Vec<u8>>,
    base: Storage,
}

impl Cache {
    pub fn store(&mut self, state_key: StateKey, v: Value, l: MoveTypeLayout) {
        // INVARIANT: All data in storage (base) is stored as is.
        let blob = v
            .simple_serialize(&l)
            .expect("Deserialization when storing a value always succeeds");
        self.base.inner.insert(state_key, blob);
    }

    pub fn patch(&mut self, identifier: u64, v: Value) {
        let mut liftings = self.liftings.borrow_mut();
        liftings.insert(identifier, v);
    }

    pub fn store_patched(&mut self, state_key: StateKey, v: Value, l: MoveTypeLayout) {
        // INVARIANT: All data in cache is patched.
        // As a result, one should call `patch` before this method.
        let blob = v
            .simple_serialize(&l)
            .expect("Deserialization when caching a value always succeeds");
        self.inner.insert(state_key, blob);
    }
}

impl ValueExchange for Cache {
    fn try_exchange(&self, value_to_exchange: Value) -> ExchangeResult<Value> {
        let mut liftings = self.liftings.borrow_mut();

        let identifier = liftings.len() as u64;
        let identifier_value = value_to_exchange.build_identifier(identifier).expect("");
        liftings.insert(identifier, value_to_exchange);
        Ok(identifier_value)
    }

    fn try_claim_back(&self, value_to_exchange: Value) -> ExchangeResult<Value> {
        let liftings = self.liftings.borrow();

        let identifier = value_to_exchange
            .as_identifier()
            .expect("Exchanged value should be convertible to identifiers");
        Ok(liftings
            .get(&identifier)
            .expect("Identifiers must always exist in the lifting map")
            .copy_value()
            .expect("Copying lifting values should never fail"))
    }
}

impl ResourceResolver for Cache {
    fn get_resource_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
        _metadata: &[Metadata],
        layout: Option<&MoveTypeLayout>,
    ) -> anyhow::Result<(Option<Vec<u8>>, usize), Error> {
        let ap = AccessPath::resource_access_path(*address, typ.clone())
            .expect("Access path for resource have to be valid");
        let state_key = StateKey::access_path(ap);

        let cached_blob = self.inner.get(&state_key);
        Ok(match cached_blob {
            Some(blob) => (Some(blob.clone()), blob.len()),
            None => {
                let (maybe_blob, size) = self
                    .base
                    .get_resource_with_metadata_and_layout(address, typ, _metadata, layout)?;
                match (layout, &maybe_blob) {
                    (Some(layout), Some(blob)) => {
                        let value = deserialize_and_exchange(blob, layout, self)
                            .expect("Deserializing a resource value should succeed");
                        let patched_blob = value
                            .simple_serialize(layout)
                            .expect("Serializing a resource value back should succeed");
                        (Some(patched_blob), size)
                    },
                    _ => (maybe_blob, size),
                }
            },
        })
    }
}

impl TableResolver for Cache {
    fn resolve_table_entry_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<Vec<u8>>, Error> {
        let state_key = StateKey::table_item((*handle).into(), key.to_vec());
        Ok(self.inner.get(&state_key).cloned().or_else(|| {
            let maybe_blob = self
                .base
                .resolve_table_entry_with_layout(handle, key, layout)
                .expect("Value should exist");
            match (layout, &maybe_blob) {
                (Some(layout), Some(blob)) => {
                    let value = deserialize_and_exchange(blob, layout, self)
                        .expect("Deserializing a table value should succeed");
                    let patched_blob = value
                        .simple_serialize(layout)
                        .expect("Serializing a table value back should succeed");
                    Some(patched_blob)
                },
                _ => maybe_blob,
            }
        }))
    }
}

fn test_layout() -> MoveTypeLayout {
    MoveTypeLayout::Struct(MoveStructLayout::Runtime(vec![
        MoveTypeLayout::Tagged(LayoutTag::AggregatorLifting, Box::new(MoveTypeLayout::U64)),
        MoveTypeLayout::Tagged(LayoutTag::AggregatorLifting, Box::new(MoveTypeLayout::U128)),
    ]))
}

fn test_struct(x: u64, y: u128) -> Value {
    Value::struct_(Struct::pack(vec![Value::u64(x), Value::u128(y)]))
}

#[test]
fn test_resource_values() {
    let key = |tag: &StructTag| -> StateKey {
        StateKey::access_path(
            AccessPath::resource_access_path(AccountAddress::ONE, tag.clone()).unwrap(),
        )
    };
    let mut cache = Cache::default();

    let foo = StructTag::from_str("0x1::foo::Foo").unwrap();
    cache.store(key(&foo), test_struct(100, 200), test_layout());

    let bar = StructTag::from_str("0x1::bar::Bar").unwrap();
    cache.store(key(&bar), test_struct(u64::MAX, u128::MAX), test_layout());

    let baz = StructTag::from_str("0x1::baz::Baz").unwrap();
    cache.patch(0, Value::u64(300));
    cache.patch(1, Value::u128(400));
    cache.store_patched(key(&baz), test_struct(0, 1), test_layout());

    assert!(cache
        .liftings
        .borrow()
        .get(&0)
        .is_some_and(|v| v.equals(&Value::u64(300)).unwrap()));
    assert!(cache
        .liftings
        .borrow()
        .get(&1)
        .is_some_and(|v| v.equals(&Value::u128(400)).unwrap()));

    // Getting foo without layout should give an actual resource.
    let (blob, _) = cache
        .get_resource_with_metadata_and_layout(&AccountAddress::ONE, &foo, &[], None)
        .unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(100, 200)).unwrap());

    // Getting foo without layout should give a patched version.
    let (blob, _) = cache
        .get_resource_with_metadata_and_layout(
            &AccountAddress::ONE,
            &foo,
            &[],
            Some(&test_layout()),
        )
        .unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(2, 3)).unwrap());
    assert!(cache
        .liftings
        .borrow()
        .get(&2)
        .is_some_and(|v| v.equals(&Value::u64(100)).unwrap()));
    assert!(cache
        .liftings
        .borrow()
        .get(&3)
        .is_some_and(|v| v.equals(&Value::u128(200)).unwrap()));

    // Repeat for Bar to check for corner cases when integers are large.

    let (blob, _) = cache
        .get_resource_with_metadata_and_layout(&AccountAddress::ONE, &bar, &[], None)
        .unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(u64::MAX, u128::MAX)).unwrap());

    let (blob, _) = cache
        .get_resource_with_metadata_and_layout(
            &AccountAddress::ONE,
            &bar,
            &[],
            Some(&test_layout()),
        )
        .unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(4, 5)).unwrap());
    assert!(cache
        .liftings
        .borrow()
        .get(&4)
        .is_some_and(|v| v.equals(&Value::u64(u64::MAX)).unwrap()));
    assert!(cache
        .liftings
        .borrow()
        .get(&5)
        .is_some_and(|v| v.equals(&Value::u128(u128::MAX)).unwrap()));

    // Baz is in cache and is already patched. We should always get the patched value.

    let (blob, _) = cache
        .get_resource_with_metadata_and_layout(&AccountAddress::ONE, &baz, &[], None)
        .unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(0, 1)).unwrap());

    let (blob, _) = cache
        .get_resource_with_metadata_and_layout(
            &AccountAddress::ONE,
            &baz,
            &[],
            Some(&test_layout()),
        )
        .unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(0, 1)).unwrap());
}

#[test]
fn test_table_items() {
    let key =
        |h: &TableHandle, k: &[u8]| -> StateKey { StateKey::table_item((*h).into(), k.to_vec()) };
    let mut cache = Cache::default();

    let foo_handle = TableHandle(AccountAddress::ONE);
    let foo_key = "foo".as_bytes();
    cache.store(
        key(&foo_handle, foo_key),
        test_struct(100, 200),
        test_layout(),
    );

    let bar_handle = TableHandle(AccountAddress::ONE);
    let bar_key = "bar".as_bytes();
    cache.patch(0, Value::u64(300));
    cache.patch(1, Value::u128(400));
    cache.store_patched(key(&bar_handle, bar_key), test_struct(0, 1), test_layout());

    assert!(cache
        .liftings
        .borrow()
        .get(&0)
        .is_some_and(|v| v.equals(&Value::u64(300)).unwrap()));
    assert!(cache
        .liftings
        .borrow()
        .get(&1)
        .is_some_and(|v| v.equals(&Value::u128(400)).unwrap()));

    // Storage access.

    let blob = cache.resolve_table_entry(&foo_handle, foo_key).unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(100, 200)).unwrap());

    let blob = cache
        .resolve_table_entry_with_layout(&foo_handle, foo_key, Some(&test_layout()))
        .unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(2, 3)).unwrap());
    assert!(cache
        .liftings
        .borrow()
        .get(&2)
        .is_some_and(|v| v.equals(&Value::u64(100)).unwrap()));
    assert!(cache
        .liftings
        .borrow()
        .get(&3)
        .is_some_and(|v| v.equals(&Value::u128(200)).unwrap()));

    // Cache access.

    let blob = cache.resolve_table_entry(&bar_handle, bar_key).unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(0, 1)).unwrap());

    let blob = cache
        .resolve_table_entry_with_layout(&bar_handle, bar_key, Some(&test_layout()))
        .unwrap();
    let value = Value::simple_deserialize(&blob.unwrap(), &test_layout()).unwrap();
    assert!(value.equals(&test_struct(0, 1)).unwrap());
}

// TODO(aggregator): Add tests for resource groups when they are supported.
