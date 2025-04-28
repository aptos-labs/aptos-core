// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    state_store::{
        errors::StateViewError, state_key::StateKey, state_slot::StateSlot,
        state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::Version,
};
use aptos_crypto::HashValue;
use bytes::Bytes;
use move_core_types::{language_storage::StructTag, move_resource::MoveResource};
#[cfg(any(test, feature = "testing"))]
use std::hash::Hash;
use std::ops::Deref;

pub mod errors;
pub mod state_key;
pub mod state_slot;
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

    /// Gets state storage usage info at epoch ending.
    fn get_usage(&self) -> StateViewResult<StateStorageUsage>;

    /// Returns the version after this view.
    fn next_version(&self) -> Version {
        // TODO(HotState):
        // Hack: This is currently only used by the HotStateOpAccumulator to decide if to refresh
        //       an already hot item.
        //       hot state promotions and evictions are not currently serialized, authenticated or
        //       charged, so it's okay for this to always return 0 if not implemented.
        0
    }

    /// Returns the version of the view.
    ///
    /// The empty "pre-genesis" state view has version None.
    fn version(&self) -> Option<Version> {
        self.next_version().checked_sub(1)
    }

    /// Gets the state slot for a given state key.
    fn get_state_slot(&self, state_key: &Self::Key) -> StateViewResult<StateSlot> {
        // TODO(HotState):
        // Hack: hot state promotions and evictions are not currently serialized, authenticated or
        //       charged, so it's okay for most type of state views to fake it.
        Ok(StateSlot::from_db_get(
            self.get_state_value(state_key)?.map(|val| (0, val)),
        ))
    }

    /// Gets the state value for a given state key.
    fn get_state_value(&self, state_key: &Self::Key) -> StateViewResult<Option<StateValue>> {
        // if not implemented, delegate to get_state_slot.
        self.get_state_slot(state_key)
            .map(StateSlot::into_state_value_opt)
    }

    /// Gets the state value bytes for a given state key.
    fn get_state_value_bytes(&self, state_key: &Self::Key) -> StateViewResult<Option<Bytes>> {
        let val_opt = self.get_state_value(state_key)?;
        Ok(val_opt.map(|val| val.bytes().clone()))
    }

    /// Checks if a state keyed by the given state key exists.
    fn contains_state_value(&self, state_key: &Self::Key) -> StateViewResult<bool> {
        self.get_state_value(state_key).map(|opt| opt.is_some())
    }
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

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        self.deref().get_usage()
    }

    fn get_state_slot(&self, state_key: &K) -> StateViewResult<StateSlot> {
        self.deref().get_state_slot(state_key)
    }

    fn get_state_value(&self, state_key: &K) -> StateViewResult<Option<StateValue>> {
        self.deref().get_state_value(state_key)
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

    fn get_state_value(&self, state_key: &Self::Key) -> StateViewResult<Option<StateValue>> {
        Ok(self.data.get(state_key).cloned())
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
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

    fn fetch_move_resource_from_group(
        state_view: &dyn StateView,
        address: &AccountAddress,
        group: &StructTag,
    ) -> StateViewResult<Option<Self>> {
        let rg = state_view
            .get_state_value_bytes(&StateKey::resource_group(address, group))?
            .map(|data| bcs::from_bytes::<std::collections::BTreeMap<StructTag, Vec<u8>>>(&data))
            .transpose()?;
        if let Some(group) = rg {
            if let Some(data) = group.get(&Self::struct_tag()) {
                return Ok(Some(bcs::from_bytes::<Self>(data)?));
            }
        }
        Ok(None)
    }
}

impl<T: MoveResource> MoveResourceExt for T {}
