// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{genesis::GENESIS_CHANGE_SET_HEAD, Account, AccountData};
use anyhow::{anyhow, bail, Result};
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{FeatureFlag, Features, OnChainConfig},
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        StateViewId, StateViewResult, TStateView,
    },
    write_set::{TransactionWrite, WriteSet},
};
use bytes::Bytes;
use move_binary_format::{deserializer::DeserializerConfig, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, language_storage::ModuleId, move_resource::MoveResource,
};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

/***************************************************************************************************
 * Traits
 *
 **************************************************************************************************/
/// Trait that extends [`TStateView`] by adding APIs for querying and modifying state values.
///
/// This trait provides a standardized interface for interacting with state stores,
/// reducing the need for users to build their own state representations or utility functions.
///
/// It is recommended that transaction-based simulations are built against this trait rather than
/// a specific state store implementation, so to maximize portability.
pub trait SimulationStateStore: TStateView<Key = StateKey> {
    /// Sets the state value associated with the given `state_key`.
    fn set_state_value(&self, state_key: StateKey, state_val: StateValue) -> Result<()>;
    /// Removes the state value associated with the given `state_key`.
    fn remove_state_value(&self, state_key: &StateKey) -> Result<()>;
    /// Applies a `WriteSet`, performing multiple state operations in batch.
    fn apply_write_set(&self, write_set: &WriteSet) -> Result<()>;

    /// Stores a Move resource at the specified address.
    ///
    /// The Rust type representing the Move resource must implement the [`MoveResource`] trait.
    fn set_resource<R>(&self, addr: AccountAddress, resource: &R) -> Result<()>
    where
        R: MoveResource + Serialize,
    {
        let state_key = StateKey::resource_typed::<R>(&addr)?;

        self.set_state_value(
            state_key,
            StateValue::new_legacy(bcs::to_bytes(resource)?.into()),
        )
    }
    /// Fetches a Move resource stored at the specified address.
    ///
    /// The Rust type representing the Move resource must implement the [`MoveResource`] trait.
    fn get_resource<R: MoveResource>(&self, addr: AccountAddress) -> Result<Option<R>> {
        let state_key = StateKey::resource_typed::<R>(&addr)?;

        match self.get_state_value_bytes(&state_key)? {
            Some(blob) => Ok(bcs::from_bytes(&blob)?),
            None => Ok(None),
        }
    }
    /// Modifies a Move resource stored at the specified address in-place.
    ///
    /// The Rust type representing the Move resource must implement the [`MoveResource`] trait.
    fn modify_resource<R, F>(&self, addr: AccountAddress, modify: F) -> Result<()>
    where
        R: MoveResource + Serialize,
        F: FnOnce(&mut R) -> Result<()>,
    {
        let mut resource = match self.get_resource(addr)? {
            Some(resource) => resource,
            None => bail!("failed to modify resource -- resource does not exist"),
        };
        modify(&mut resource)?;
        self.set_resource(addr, &resource)
    }

    /// Sets an on-chain config.
    ///
    /// The Rust type representing the on-chain config must implement the [`OnChainConfig`] trait.
    fn set_on_chain_config<C>(&self, config: &C) -> Result<()>
    where
        C: OnChainConfig + Serialize,
    {
        self.set_state_value(
            StateKey::on_chain_config::<C>()?,
            StateValue::new_legacy(bcs::to_bytes(&config)?.into()),
        )
    }
    /// Gets an on-chain config.
    ///
    /// The Rust type representing the on-chain config must implement the [`OnChainConfig`] trait.
    fn get_on_chain_config<C>(&self) -> Result<C>
    where
        Self: Sized,
        C: OnChainConfig,
    {
        C::fetch_config(self).ok_or_else(|| {
            anyhow!(
                "failed to fetch on-chain config: {:?}",
                std::any::type_name::<C>()
            )
        })
    }
    /// Modifies an on-chain config in-place.
    ///
    /// The Rust type representing the on-chain config must implement the [`OnChainConfig`] trait.
    fn modify_on_chain_config<C, F>(&self, modify: F) -> Result<()>
    where
        Self: Sized,
        C: OnChainConfig + Serialize,
        F: FnOnce(&mut C) -> Result<()>,
    {
        let mut config = self.get_on_chain_config::<C>()?;
        modify(&mut config)?;
        self.set_on_chain_config(&config)
    }

    /// Stores a compiled Move module, with the module being already serialized into a blob.
    fn add_module_blob(&self, module_id: &ModuleId, blob: impl Into<Bytes>) -> Result<()> {
        self.set_state_value(
            StateKey::module_id(module_id),
            StateValue::new_legacy(blob.into()),
        )
    }
    /// Stores a compiled Move module.
    fn add_module(&self, module: &CompiledModule) -> Result<()> {
        let mut blob = vec![];
        module.serialize(&mut blob)?;

        self.set_state_value(
            StateKey::module_id(&module.self_id()),
            StateValue::new_legacy(blob.into()),
        )
    }
    /// Gets a Move module.
    fn get_module(&self, module_id: &ModuleId) -> Result<Option<CompiledModule>>
    where
        Self: Sized,
    {
        let features = self.get_features()?;
        let deserializer_config = DeserializerConfig::new(
            features.get_max_binary_format_version(),
            features.get_max_identifier_size(),
        );

        let blob = match self.get_state_value_bytes(&StateKey::module_id(module_id))? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let m = CompiledModule::deserialize_with_config(&blob, &deserializer_config)?;

        Ok(Some(m))
    }

    /// Gets the [`ChainId`] resource that is used to identify the blockchain network.
    fn get_chain_id(&self) -> Result<ChainId>
    where
        Self: Sized,
    {
        self.get_on_chain_config()
    }

    /// Sets the [`ChainId`] resource that is used to identify the blockchain network.
    fn set_chain_id(&self, chain_id: ChainId) -> Result<()> {
        self.set_on_chain_config(&chain_id)
    }

    /// Gets the on-chain feature flags.
    fn get_features(&self) -> Result<Features>
    where
        Self: Sized,
    {
        self.get_on_chain_config()
    }

    /// Sets the on-chain feature flags.
    fn set_features(&self, features: Features) -> Result<()> {
        let bytes = bcs::to_bytes(&features)?;

        self.set_state_value(
            StateKey::resource(Features::address(), &Features::struct_tag()).unwrap(),
            StateValue::new_legacy(bytes.into()),
        )
    }

    /// Adds the given account_data to the storage.
    fn add_account_data(&self, account_data: &AccountData) -> Result<()> {
        let write_set = account_data.to_writeset();
        self.apply_write_set(&write_set)
    }

    /// Creates and stores a new account with the given balance and sequence number.
    fn store_and_fund_account(
        &self,
        account: Account,
        balance: u64,
        seq_num: u64,
    ) -> Result<AccountData>
    where
        Self: Sized,
    {
        let features: Features = self.get_on_chain_config().unwrap_or_default();
        let use_fa_balance = features.is_enabled(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
        let use_concurrent_balance =
            features.is_enabled(FeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE);

        let data = AccountData::with_account(
            account,
            balance,
            seq_num,
            use_fa_balance,
            use_concurrent_balance,
        );

        self.add_account_data(&data)?;
        Ok(data)
    }
}

/***************************************************************************************************
 * Empty State View
 *
 **************************************************************************************************/
/// Represents a state view that contains no state.
///
/// This is useful as a base state for situations where no prior state exists.
#[derive(Debug, Clone)]
pub struct EmptyStateView;

impl TStateView for EmptyStateView {
    type Key = StateKey;

    fn get_state_value(&self, _state_key: &Self::Key) -> StateViewResult<Option<StateValue>> {
        Ok(None)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(StateStorageUsage::Untracked)
    }

    fn contains_state_value(&self, _state_key: &Self::Key) -> StateViewResult<bool> {
        Ok(false)
    }
}

/***************************************************************************************************
 * Either State View
 *
 **************************************************************************************************/
/// Provides a way to dispatch between two types of state views.
#[derive(Debug, Clone)]
pub enum EitherStateView<L, R> {
    Left(L),
    Right(R),
}

impl<L, R, K> TStateView for EitherStateView<L, R>
where
    L: TStateView<Key = K>,
    R: TStateView<Key = K>,
{
    type Key = K;

    fn id(&self) -> StateViewId {
        match self {
            Self::Left(l) => l.id(),
            Self::Right(r) => r.id(),
        }
    }

    fn get_state_value(&self, state_key: &Self::Key) -> StateViewResult<Option<StateValue>> {
        match self {
            Self::Left(l) => l.get_state_value(state_key),
            Self::Right(r) => r.get_state_value(state_key),
        }
    }

    fn get_state_value_bytes(&self, state_key: &Self::Key) -> StateViewResult<Option<Bytes>> {
        match self {
            Self::Left(l) => l.get_state_value_bytes(state_key),
            Self::Right(r) => r.get_state_value_bytes(state_key),
        }
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        match self {
            Self::Left(l) => l.get_usage(),
            Self::Right(r) => r.get_usage(),
        }
    }

    fn contains_state_value(&self, state_key: &Self::Key) -> StateViewResult<bool> {
        match self {
            Self::Left(l) => l.contains_state_value(state_key),
            Self::Right(r) => r.contains_state_value(state_key),
        }
    }
}

/***************************************************************************************************
 * Delta State Store
 *
 **************************************************************************************************/
/// A state storage that allows changes to be stacked on top of a base state view.
///
/// This is useful for staging reversible state changes or performing simulations on top of
/// remote states.
pub struct DeltaStateStore<V> {
    base: V,
    states: RwLock<HashMap<StateKey, Option<StateValue>>>,
}

impl<V> TStateView for DeltaStateStore<V>
where
    V: TStateView<Key = StateKey>,
{
    type Key = StateKey;

    fn get_state_value(&self, state_key: &Self::Key) -> StateViewResult<Option<StateValue>> {
        if let Some(res) = self.states.read().get(state_key) {
            return Ok(res.clone());
        }
        self.base.get_state_value(state_key)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(StateStorageUsage::Untracked)
    }

    fn contains_state_value(&self, state_key: &Self::Key) -> StateViewResult<bool> {
        match self.states.read().get(state_key) {
            Some(Some(_)) => return Ok(true),
            Some(None) => return Ok(false),
            None => (),
        }

        self.base.contains_state_value(state_key)
    }
}

impl<V> SimulationStateStore for DeltaStateStore<V>
where
    V: TStateView<Key = StateKey>,
{
    fn set_state_value(&self, state_key: StateKey, state_val: StateValue) -> Result<()> {
        self.states.write().insert(state_key, Some(state_val));
        Ok(())
    }

    fn remove_state_value(&self, state_key: &StateKey) -> Result<()> {
        let mut states = self.states.write();

        match states.get_mut(state_key) {
            Some(val) => *val = None,
            None => {
                states.insert(state_key.clone(), None);
            },
        }

        Ok(())
    }

    fn apply_write_set(&self, write_set: &WriteSet) -> Result<()> {
        let mut states = self.states.write();

        for (state_key, write_op) in write_set.write_op_iter() {
            match write_op.as_state_value() {
                None => match states.get_mut(state_key) {
                    Some(val) => *val = None,
                    None => {
                        states.insert(state_key.clone(), None);
                    },
                },
                Some(state_val) => {
                    states.insert(state_key.clone(), Some(state_val));
                },
            }
        }

        Ok(())
    }
}

impl<V> DeltaStateStore<V> {
    /// Creates a new [`DeltaStateStore`] with a given base state view and no additional state
    /// changes.
    pub fn new_with_base(base: V) -> Self {
        Self {
            base,
            states: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a new [`DeltaStateStore`] with a given base state view and the given state
    /// values.
    pub fn new_with_base_and_state_values(
        base: V,
        state_vals: impl IntoIterator<Item = (StateKey, StateValue)>,
    ) -> Self {
        Self {
            base,
            states: RwLock::new(state_vals.into_iter().map(|(k, v)| (k, Some(v))).collect()),
        }
    }
}

impl<V> Clone for DeltaStateStore<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            states: RwLock::new(self.states.read().clone()),
        }
    }
}

/***************************************************************************************************
 * In Memory State Store
 *
 **************************************************************************************************/
/// A simple in-memory key-value store, intended for use in simulations.
///
/// Currently implemented as an alias for [`DeltaStateStore<EmptyStateView>`], but this is an
/// implementation detail and may change in the future.
/// API compatibility will be maintained however.
pub type InMemoryStateStore = DeltaStateStore<EmptyStateView>;

impl InMemoryStateStore {
    /// Creates a new empty [`InMemoryStateStore`].
    pub fn new() -> Self {
        Self::new_with_base(EmptyStateView)
    }

    /// Creates a new [`InMemoryStateStore`] with the given values.
    pub fn new_with_state_values(
        state_vals: impl IntoIterator<Item = (StateKey, StateValue)>,
    ) -> Self {
        Self::new_with_base_and_state_values(EmptyStateView, state_vals)
    }

    /// Creates a new [`InMemoryStateStore`] from a given Aptos network genesis.
    pub fn from_genesis(write_set: &WriteSet, chain_id: ChainId) -> Self {
        let state_store = Self::new();

        state_store.set_chain_id(chain_id).unwrap();
        state_store.apply_write_set(write_set).unwrap();

        state_store
    }

    /// Creates a new [`InMemoryStateStore`] from the genesis built from the current development branch.
    pub fn from_head_genesis() -> Self {
        Self::from_genesis(GENESIS_CHANGE_SET_HEAD.write_set(), ChainId::test())
    }

    /// Converts to a [`BTreeMap`] of state keys and values, which can be useful for serialization.
    pub fn to_btree_map(&self) -> BTreeMap<StateKey, StateValue> {
        self.states
            .read()
            .iter()
            .flat_map(|(k, v)| v.as_ref().map(|v| (k.clone(), v.clone())))
            .collect()
    }
}

impl Default for InMemoryStateStore {
    fn default() -> Self {
        Self::new()
    }
}

/***************************************************************************************************
 * Tests
 *
 **************************************************************************************************/
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{collection::vec, prelude::*};
    use std::collections::BTreeMap;

    /// Strategy to generate a [`StateKey`] for testing -- currently a byte array of
    /// max length 4.
    fn key_strategy() -> impl Strategy<Value = StateKey> {
        vec(any::<u8>(), 0..=4).prop_map(|bytes| StateKey::raw(&bytes))
    }

    /// Strategy to generate a [`StateValue`] for testing -- currently a byte array of
    /// max length 4.
    fn val_strategy() -> impl Strategy<Value = StateValue> {
        vec(any::<u8>(), 0..=4).prop_map(|bytes| StateValue::new_legacy(bytes.into()))
    }

    /// Represents a state store operation for testing purposes.
    #[derive(Debug, Clone)]
    enum Op {
        Read(StateKey),
        Write(StateKey, StateValue),
        Delete(StateKey),
    }

    impl Arbitrary for Op {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                key_strategy().prop_map(Op::Read),
                (key_strategy(), val_strategy()).prop_map(|(key, val)| { Op::Write(key, val) }),
                key_strategy().prop_map(Op::Delete)
            ]
            .boxed()
        }
    }

    proptest! {
        /// Verifies that [`DeltaStateStore`] maintains the same behavior as if all writes
        /// were directly applied to the base storage.
        #[test]
        fn test_delta_state_store_has_correct_semantics(
            initial_state in vec((key_strategy(), val_strategy()), 0..=10),
            ops in vec(any::<Op>(), 1..100),
        ) {
            use Op::*;

            let base = InMemoryStateStore::new_with_state_values(initial_state);
            let raw = base.clone();
            let delta = DeltaStateStore::new_with_base(&base);

            for op in ops {
                match op {
                    Write(key, val) => {
                        raw.set_state_value(key.clone(), val.clone()).unwrap();
                        delta.set_state_value(key, val).unwrap();
                    }
                    Delete(key) => {
                        raw.remove_state_value(&key).unwrap();
                        delta.remove_state_value(&key).unwrap();
                    }
                    Read(key) => {
                        assert_eq!(raw.get_state_value(&key).unwrap(), delta.get_state_value(&key).unwrap());
                    }
                }
            }
        }

        /// Verifies that [`DeltaStateStore`] maintains the same behavior as if a write set
        /// were directly applied to the base storage.
        #[test]
        fn test_delta_state_store_has_correct_semantics_for_write_set(
            initial_state in vec((key_strategy(), val_strategy()), 0..=10),
            write_set_ops in vec((key_strategy(), prop::option::of(val_strategy())), 1..100),
            reads in vec(key_strategy(), 1..30),
        ) {
            let base = InMemoryStateStore::new_with_state_values(initial_state);
            let raw = base.clone();
            let delta = DeltaStateStore::new_with_base(&base);

            let write_set = WriteSet::new_for_test(write_set_ops);
            raw.apply_write_set(&write_set).unwrap();
            delta.apply_write_set(&write_set).unwrap();

            for key in reads {
                assert_eq!(raw.get_state_value(&key).unwrap(), delta.get_state_value(&key).unwrap());
            }
        }

        /// Verifies that [`InMemoryStateStore`] behaves exactly like a [`BTreeMap`],
        /// a.k.a. a key-value storage.
        #[test]
        fn test_in_memory_state_store_behaves_like_btree_map(ops in vec(any::<Op>(), 1..100)) {
            use Op::*;

            let mut map = BTreeMap::new();
            let store1 = InMemoryStateStore::new();
            let store2 = InMemoryStateStore::new();

            // For state store 1, we apply all operations as a write set
            // Read operations are ignored
            store1.apply_write_set(&WriteSet::new_for_test(ops.clone().into_iter().filter_map(|op| match op {
                Write(key, val) => Some((key, Some(val))),
                Delete(key) => Some((key, None)),
                Read(_) => None,
            }))).unwrap();

            // For state store 2 & the reference BTreeMap, we apply the operations one by one
            for op in ops {
                match op {
                    Write(key, val) => {
                        map.insert(key.clone(), val.clone());
                        store2.set_state_value(key, val).unwrap();
                    }
                    Delete(key) => {
                        map.remove(&key);
                        store2.remove_state_value(&key).unwrap();
                    }
                    Read(key) => {
                        assert_eq!(map.get(&key), store2.get_state_value(&key).unwrap().as_ref());
                    }
                }

                assert_eq!(map, store2.to_btree_map());
            }

            assert_eq!(map, store1.to_btree_map());
        }
    }
}
