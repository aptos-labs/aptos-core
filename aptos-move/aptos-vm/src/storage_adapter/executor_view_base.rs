// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    aggregator_extension::AggregatorID,
    resolver::{AggregatorReadMode, TAggregatorView},
};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use aptos_vm_types::resolver::{StateStorageView, TModuleView, TResourceView};
use move_core_types::value::MoveTypeLayout;

/// Adapter to convert a `StateView` into an `ExecutorView`.
pub struct ExecutorViewBase<'s, S>(&'s S);

impl<'s, S: StateView> ExecutorViewBase<'s, S> {
    pub(crate) fn new(state_view: &'s S) -> Self {
        Self(state_view)
    }
}

pub trait AsExecutorView<S> {
    fn as_executor_view(&self) -> ExecutorViewBase<S>;
}

impl<S: StateView> AsExecutorView<S> for S {
    fn as_executor_view(&self) -> ExecutorViewBase<S> {
        ExecutorViewBase::new(self)
    }
}

impl<'s, S: StateView> TAggregatorView for ExecutorViewBase<'s, S> {
    type IdentifierV1 = StateKey;
    type IdentifierV2 = AggregatorID;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::IdentifierV1,
        // Reading from StateView can be in precise mode only.
        _mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        self.0.get_state_value(state_key)
    }
}

impl<'s, S: StateView> TResourceView for ExecutorViewBase<'s, S> {
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

impl<'s, S: StateView> TModuleView for ExecutorViewBase<'s, S> {
    type Key = StateKey;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.0.get_state_value(state_key)
    }
}

impl<'s, S: StateView> StateStorageView for ExecutorViewBase<'s, S> {
    fn id(&self) -> StateViewId {
        self.0.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.0.get_usage()
    }
}
