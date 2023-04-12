// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_state_view::{StateViewId, TStateView};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TransactionWrite,
};
use aptos_vm_types::{
    change_set::WriteChangeSet,
    effects::Op,
    remote_cache::{TRemoteCache, TStateViewWithRemoteCache},
    write::{AptosModuleRef, AptosResourceRef},
};

pub struct DeltaStateView<'a, 'b, S> {
    base: &'a S,
    writes: &'b WriteChangeSet,
    // TODO: add deltas here!
}

impl<'a, 'b, S> DeltaStateView<'a, 'b, S> {
    pub fn new(base: &'a S, writes: &'b WriteChangeSet) -> Self {
        Self { base, writes }
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
        match self.writes.get(state_key) {
            Some(write) => Ok(write.clone().into_write_op()?.as_state_value()),
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

    fn get_cached_module(&self, state_key: &Self::Key) -> anyhow::Result<Option<AptosModuleRef>> {
        match self.writes.get(state_key) {
            Some(_) => Ok(None),
            // Some(write_op) => Ok(match write_op {
            //     Op::Creation(write) | Op::Modification(write) => None,
            //     Op::Deletion => None,
            // }),
            None => self.base.get_cached_module(state_key),
        }
    }

    fn get_cached_resource(
        &self,
        state_key: &Self::Key,
    ) -> anyhow::Result<Option<AptosResourceRef>> {
        match self.writes.get(state_key) {
            Some(_) => Ok(None),
            // Some(write_op) => Ok(match write_op {
            //     Op::Creation(write) | Op::Modification(write) => None,
            //     Op::Deletion => None,
            // }),
            None => self.base.get_cached_resource(state_key),
        }
    }
}
