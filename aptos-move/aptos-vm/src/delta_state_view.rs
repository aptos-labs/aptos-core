// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    resource::TransactionWrite,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::WriteSet,
};
use aptos_types::resource::AptosResource;
use aptos_vm_view::types::{TRemoteCache, TStateViewWithRemoteCache};

pub struct DeltaStateView<'a, 'b, S> {
    base: &'a S,
    write_set: &'b WriteSet,
}

impl<'a, 'b, S> DeltaStateView<'a, 'b, S> {
    pub fn new(base: &'a S, write_set: &'b WriteSet) -> Self {
        Self { base, write_set }
    }
}

impl<'a, 'b, S> TStateViewWithRemoteCache for DeltaStateView<'a, 'b, S>
where
    S: TStateViewWithRemoteCache<CommonKey = StateKey>,
{
    type CommonKey = StateKey;
}

impl<'a, 'b, S> TStateView for DeltaStateView<'a, 'b, S>
where
    S: TStateView<Key = StateKey>,
{
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        match self.write_set.get(state_key) {
            Some(write_op) => Ok(write_op.as_state_value()),
            None => self.base.get_state_value(state_key),
        }
    }

    fn is_genesis(&self) -> bool {
        self.base.is_genesis()
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        // TODO(Gas): Check if this is correct
        self.base.get_usage()
    }
}

impl<'a, 'b, S> TRemoteCache for DeltaStateView<'a, 'b, S>
    where
        S: TRemoteCache<Key = StateKey>,
{
    type Key = StateKey;

    fn get_cached_module(&self, state_key: &Self::Key) -> Result<Option<Vec<u8>>> {
        todo!()
    }

    fn get_cached_resource(&self, state_key: &Self::Key) -> Result<Option<AptosResource>> {
        match self.write_set.get(state_key) {
            Some(write_op) => Ok(write_op.as_aptos_resource()),
            None => self.base.get_cached_resource(state_key),
        }
    }
}
