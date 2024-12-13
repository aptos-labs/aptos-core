// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    state_store::{
        errors::StateViewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        state_value::StateValue,
    },
    transaction::Version,
};
use aptos_crypto::HashValue;
use bytes::Bytes;
use move_core_types::move_resource::MoveResource;
#[cfg(any(test, feature = "testing"))]
use std::hash::Hash;
use std::ops::Deref;

pub mod errors;
pub mod state_key;
pub mod state_storage_usage;
pub mod state_value;
pub mod table;

pub type StateViewResult<T, E = StateViewError> = std::result::Result<T, E>;

/// A trait that defines a read-only snapshot of the global state. It is passed to the VM for
/// transaction execution, during which the VM is guaranteed to read anything at the given state.
pub trait TStateView {
    type Key;

    /// For logging and debugging purpose, identifies what this view is for.
    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    /// Gets the state value bytes for a given state key.
    fn get_state_value_bytes(&self, state_key: &Self::Key) -> StateViewResult<Option<Bytes>> {
        let val_opt = self.get_state_value(state_key)?;
        Ok(val_opt.map(|val| val.bytes().clone()))
    }

    /// Gets the state value for a given state key.
    fn get_state_value(&self, state_key: &Self::Key) -> StateViewResult<Option<StateValue>>;

    /// Get state storage usage info at epoch ending.
    fn get_usage(&self) -> StateViewResult<StateStorageUsage>;
}

pub trait StateView: TStateView<Key = StateKey> {}

impl<T: TStateView<Key = StateKey>> StateView for T {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StateViewId {
    /// State-sync applying a chunk of transactions.
    ChunkExecution {
        first_version: Version,
    },
    /// LEC applying a block.
    BlockExecution {
        block_id: HashValue,
    },
    /// VmValidator verifying incoming transaction.
    TransactionValidation {
        base_version: Version,
    },
    /// For test, db-bootstrapper, etc. Usually not aimed to pass to VM.
    Miscellaneous,
    Replay,
}

impl<R, S, K> TStateView for R
where
    R: Deref<Target = S>,
    S: TStateView<Key = K>,
{
    type Key = K;

    fn id(&self) -> StateViewId {
        self.deref().id()
    }

    fn get_state_value(&self, state_key: &K) -> StateViewResult<Option<StateValue>> {
        self.deref().get_state_value(state_key)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        self.deref().get_usage()
    }
}

/// Test-only basic [StateView] implementation with generic keys.
#[cfg(any(test, feature = "testing"))]
pub struct MockStateView<K> {
    data: std::collections::HashMap<K, StateValue>,
}

#[cfg(any(test, feature = "testing"))]
impl<K> MockStateView<K> {
    pub fn empty() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }

    pub fn new(data: std::collections::HashMap<K, StateValue>) -> Self {
        Self { data }
    }
}

#[cfg(any(test, feature = "testing"))]
impl<K: Clone + Eq + Hash> TStateView for MockStateView<K> {
    type Key = K;

    fn get_state_value(
        &self,
        state_key: &Self::Key,
    ) -> StateViewResult<Option<StateValue>, StateViewError> {
        Ok(self.data.get(state_key).cloned())
    }

    fn get_usage(&self) -> std::result::Result<StateStorageUsage, StateViewError> {
        unimplemented!("Irrelevant for tests");
    }
}

pub trait MoveResourceExt: MoveResource {
    fn fetch_move_resource(
        state_view: &dyn StateView,
        address: &AccountAddress,
    ) -> StateViewResult<Option<Self>> {
        let state_key = StateKey::resource_typed::<Self>(address)?;
        Ok(state_view
            .get_state_value_bytes(&state_key)?
            .map(|bytes| bcs::from_bytes(&bytes))
            .transpose()?)
    }
}

impl<T: MoveResource> MoveResourceExt for T {}
