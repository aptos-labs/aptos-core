// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{aggregator_extension::AggregatorID, resolver::TAggregatorResolver};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use aptos_vm_types::resolver::{StateStorageResolver, TModuleResolver, TResourceResolver};
use move_core_types::value::MoveTypeLayout;

/// Adapter to convert a `StateView` into an `ExecutorResolver`.
pub struct StateViewAdapter<'s, S>(pub &'s S);

impl<'s, S: StateView> TAggregatorResolver for StateViewAdapter<'s, S> {
    type Key = AggregatorID;

    fn get_aggregator_v1_state_value(&self, id: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.0.get_state_value(id.as_state_key())
    }
}

impl<'s, S: StateView> TResourceResolver for StateViewAdapter<'s, S> {
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        _maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<StateValue>> {
        self.0.get_state_value(state_key)
    }
}

impl<'s, S: StateView> TModuleResolver for StateViewAdapter<'s, S> {
    type Key = StateKey;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.0.get_state_value(state_key)
    }
}

impl<'s, S: StateView> StateStorageResolver for StateViewAdapter<'s, S> {
    fn id(&self) -> StateViewId {
        self.0.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.0.get_usage()
    }
}
