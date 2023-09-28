// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    resolver::{AggregatorReadMode, TAggregatorView},
    types::{AggregatorID, AggregatorValue},
};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use aptos_vm_types::resolver::{StateStorageView, TModuleView, TResourceView};
use move_core_types::value::MoveTypeLayout;
use std::sync::atomic::{AtomicU32, Ordering};

/// Adapter to convert a `StateView` into an `ExecutorView`.
pub struct ExecutorViewBase<'s, S> {
    base: &'s S,
    // Because aggregators V2 replace identifiers, this counter serves as
    // ID generator. We use atomic so that the adapter can be used in
    // concurrent setting, plus the adapter is not supposed to be used
    // for block execution.
    counter: AtomicU32,
}

impl<'s, S: StateView> ExecutorViewBase<'s, S> {
    pub(crate) fn new(base: &'s S) -> Self {
        Self {
            base,
            counter: AtomicU32::new(0),
        }
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
        self.base.get_state_value(state_key)
    }

    fn generate_aggregator_v2_id(&self) -> Self::IdentifierV2 {
        (self.counter.fetch_add(1, Ordering::SeqCst) as u64).into()
    }

    fn get_aggregator_v2_value(
        &self,
        _id: &Self::IdentifierV2,
        _mode: AggregatorReadMode,
    ) -> anyhow::Result<AggregatorValue> {
        unimplemented!()
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
        self.base.get_state_value(state_key)
    }
}

impl<'s, S: StateView> TModuleView for ExecutorViewBase<'s, S> {
    type Key = StateKey;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        self.base.get_state_value(state_key)
    }
}

impl<'s, S: StateView> StateStorageView for ExecutorViewBase<'s, S> {
    fn id(&self) -> StateViewId {
        self.base.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.base.get_usage()
    }
}
