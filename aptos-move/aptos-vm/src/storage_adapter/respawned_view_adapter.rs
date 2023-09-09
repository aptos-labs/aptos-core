// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{aggregator_extension::AggregatorID, resolver::TAggregatorResolver};
use aptos_state_view::StateViewId;
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TransactionWrite,
};
use aptos_vm_types::{
    change_set::VMChangeSet,
    resolver::{ExecutorResolver, StateStorageResolver, TModuleResolver, TResourceResolver},
};
use move_core_types::value::MoveTypeLayout;

/// Adapter to allow resolving the calls to `ExecutorResolver` via change set.
pub struct RespawnedViewAdapter<'r> {
    base: &'r dyn ExecutorResolver,
    pub(crate) change_set: VMChangeSet,
}

impl<'r> RespawnedViewAdapter<'r> {
    pub(crate) fn new(base: &'r dyn ExecutorResolver, change_set: VMChangeSet) -> Self {
        Self { base, change_set }
    }
}

impl<'r> TAggregatorResolver for RespawnedViewAdapter<'r> {
    type Key = AggregatorID;

    fn get_aggregator_v1_state_value(&self, id: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        match self.change_set.aggregator_v1_delta_set().get(id) {
            Some(delta_op) => Ok(self
                .base
                .try_convert_aggregator_v2_delta_into_write_op(id, delta_op)?
                .as_state_value()),
            None => match self.change_set.aggregator_v1_write_set().get(id) {
                Some(write_op) => Ok(write_op.as_state_value()),
                None => self.base.get_aggregator_v1_state_value(id),
            },
        }
    }
}

impl<'r> TResourceResolver for RespawnedViewAdapter<'r> {
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<StateValue>> {
        match self.change_set.resource_write_set().get(state_key) {
            Some(write_op) => Ok(write_op.as_state_value()),
            None => self.base.get_resource_state_value(state_key, maybe_layout),
        }
    }
}

impl<'r> TModuleResolver for RespawnedViewAdapter<'r> {
    type Key = StateKey;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        match self.change_set.module_write_set().get(state_key) {
            Some(write_op) => Ok(write_op.as_state_value()),
            None => self.base.get_module_state_value(state_key),
        }
    }
}

impl<'r> StateStorageResolver for RespawnedViewAdapter<'r> {
    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        anyhow::bail!("Unexpected access to get_usage()")
    }
}
