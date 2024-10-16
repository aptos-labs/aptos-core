// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Support for mocking the Aptos data store.

use crate::account::AccountData;
use aptos_types::{
    account_config::CoinInfoResource,
    on_chain_config::{Features, OnChainConfig},
    state_store::{
        errors::StateviewError, in_memory_state_view::InMemoryStateView, state_key::StateKey,
        state_storage_usage::StateStorageUsage, state_value::StateValue, TStateView,
    },
    transaction::ChangeSet,
    write_set::{TransactionWrite, WriteSet},
    AptosCoinType,
};
use aptos_vm_genesis::{
    generate_genesis_change_set_for_mainnet, generate_genesis_change_set_for_testing,
    GenesisOptions,
};
use move_core_types::language_storage::ModuleId;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dummy genesis ChangeSet for testing
pub static GENESIS_CHANGE_SET_HEAD: Lazy<ChangeSet> =
    Lazy::new(|| generate_genesis_change_set_for_testing(GenesisOptions::Head));

pub static GENESIS_CHANGE_SET_TESTNET: Lazy<ChangeSet> =
    Lazy::new(|| generate_genesis_change_set_for_testing(GenesisOptions::Testnet));

pub static GENESIS_CHANGE_SET_MAINNET: Lazy<ChangeSet> =
    Lazy::new(|| generate_genesis_change_set_for_mainnet(GenesisOptions::Mainnet));

/// An in-memory implementation of `StateView` and `ExecutorView` for the VM.
///
/// Tests use this to set up state, and pass in a reference to the cache whenever a `StateView` or
/// `ExecutorView` is needed.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FakeDataStore {
    state_data: HashMap<StateKey, StateValue>,
}

impl FakeDataStore {
    /// Creates a new `FakeDataStore` with the provided initial data.
    pub fn new(data: HashMap<StateKey, Vec<u8>>) -> Self {
        FakeDataStore {
            state_data: data
                .into_iter()
                .map(|(k, v)| (k, StateValue::new_legacy(v.into())))
                .collect(),
        }
    }

    /// Creates a new `FakeDataStore` with the provided initial data.
    pub fn new_with_state_value(data: HashMap<StateKey, StateValue>) -> Self {
        FakeDataStore { state_data: data }
    }

    /// Adds a [`WriteSet`] to this data store.
    pub fn add_write_set(&mut self, write_set: &WriteSet) {
        for (state_key, write_op) in write_set {
            match write_op.as_state_value() {
                None => self.remove(state_key),
                Some(state_value) => self.set(state_key.clone(), state_value),
            };
        }
    }

    /// Sets a `(key, bytes)` pair within this data store. Wraps `bytes` in StateValue::new_legacy().
    ///
    /// Returns the previous data if the key was occupied.
    pub fn set_legacy(&mut self, state_key: StateKey, bytes: Vec<u8>) -> Option<StateValue> {
        self.state_data
            .insert(state_key, StateValue::new_legacy(bytes.into()))
    }

    /// Sets a (key, value) pair within this data store.
    ///
    /// Returns the previous data if the key was occupied.
    pub fn set(&mut self, state_key: StateKey, state_value: StateValue) -> Option<StateValue> {
        self.state_data.insert(state_key, state_value)
    }

    /// Checks whether the state_key is in this data store
    ///
    pub fn contains_key(&self, state_key: &StateKey) -> bool {
        self.state_data.contains_key(state_key)
    }

    /// Deletes a key from this data store.
    ///
    /// Returns the previous data if the key was occupied.
    pub fn remove(&mut self, state_key: &StateKey) -> Option<StateValue> {
        self.state_data.remove(state_key)
    }

    /// Adds an [`AccountData`] to this data store.
    pub fn add_account_data(&mut self, account_data: &AccountData) {
        let write_set = account_data.to_writeset();
        self.add_write_set(&write_set)
    }

    /// Adds CoinInfo to this data store.
    pub fn add_coin_info(&mut self) {
        let coin_info = CoinInfoResource::<AptosCoinType>::random(u128::MAX);
        let write_set = coin_info.to_writeset(0).expect("access path in test");
        self.add_write_set(&write_set)
    }

    /// Adds a `CompiledModule` to this data store.
    ///
    /// Does not do any sort of verification on the module.
    pub fn add_module(&mut self, module_id: &ModuleId, blob: Vec<u8>) {
        self.set(
            StateKey::module_id(module_id),
            StateValue::new_legacy(blob.into()),
        );
    }

    pub fn set_features(&mut self, features: Features) {
        let bytes = bcs::to_bytes(&features).expect("Features should always be serializable");
        self.set(
            StateKey::resource(Features::address(), &Features::struct_tag()).unwrap(),
            StateValue::new_legacy(bytes.into()),
        );
    }
}

// This is used by the `execute_block` API.
impl TStateView for FakeDataStore {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>, StateviewError> {
        Ok(self.state_data.get(state_key).cloned())
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateviewError> {
        let mut usage = StateStorageUsage::new_untracked();
        for (k, v) in self.state_data.iter() {
            usage.add_item(k.size() + v.size())
        }
        Ok(usage)
    }

    fn as_in_memory_state_view(&self) -> InMemoryStateView {
        InMemoryStateView::new(self.state_data.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_types::on_chain_config::{FeatureFlag, Features};
    use claims::*;

    #[test]
    fn test_features_can_be_set() {
        let mut data_store = FakeDataStore::default();
        assert_none!(Features::fetch_config(&data_store));

        data_store.set_features(Features::default());
        let features = assert_some!(Features::fetch_config(&data_store));
        assert_eq!(features, Features::default())
    }

    #[test]
    fn test_features_can_be_reset() {
        use claims::*;

        let mut data_store = FakeDataStore::default();
        data_store.add_write_set(GENESIS_CHANGE_SET_HEAD.write_set());

        // Reset the feature.
        let mut features = assert_some!(Features::fetch_config(&data_store));
        assert!(features.is_enabled(FeatureFlag::STORAGE_SLOT_METADATA));
        features.disable(FeatureFlag::STORAGE_SLOT_METADATA);
        data_store.set_features(features.clone());

        let reset_features = assert_some!(Features::fetch_config(&data_store));
        assert!(!reset_features.is_enabled(FeatureFlag::STORAGE_SLOT_METADATA));
        assert_eq!(reset_features, features)
    }
}
