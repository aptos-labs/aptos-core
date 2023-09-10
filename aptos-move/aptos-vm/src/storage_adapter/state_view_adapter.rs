// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{aggregator_extension::AggregatorID, resolver::TAggregatorView};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use aptos_vm_types::resolver::{StateStorageView, TModuleView, TResourceView};
use move_core_types::value::MoveTypeLayout;

/// Adapter to convert a `StateView` into an `ExecutorView`.
pub struct ExecutorViewAdapter<'s, S>(&'s S);

impl<'s, S: StateView> ExecutorViewAdapter<'s, S> {
    pub(crate) fn new(state_view: &'s S) -> Self {
        Self(state_view)
    }
}

pub trait AsExecutorView<S> {
    fn as_executor_view(&self) -> ExecutorViewAdapter<S>;
}

impl<S: StateView> AsExecutorView<S> for S {
    fn as_executor_view(&self) -> ExecutorViewAdapter<S> {
        ExecutorViewAdapter::new(self)
    }
}

impl<'s, S: StateView> TAggregatorView for ExecutorViewAdapter<'s, S> {
    type Key = AggregatorID;

    fn get_aggregator_v1_state_value(&self, id: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.0.get_state_value(id.as_state_key())
    }
}

impl<'s, S: StateView> TResourceView for ExecutorViewAdapter<'s, S> {
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

impl<'s, S: StateView> TModuleView for ExecutorViewAdapter<'s, S> {
    type Key = StateKey;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.0.get_state_value(state_key)
    }
}

impl<'s, S: StateView> StateStorageView for ExecutorViewAdapter<'s, S> {
    fn id(&self) -> StateViewId {
        self.0.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.0.get_usage()
    }
}
