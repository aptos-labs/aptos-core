// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    bounded_math::SignedU128,
    resolver::TDelayedFieldView,
    types::{DelayedFieldID, DelayedFieldValue, DelayedFieldsSpeculativeError, PanicOr},
};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::state_store::{
    state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
};
use aptos_vm_types::resolver::{StateStorageView, TModuleView, TResourceView};
use move_core_types::value::MoveTypeLayout;
use std::sync::atomic::{AtomicU32, Ordering};


pub trait AsExecutorView<S> {
    fn as_executor_view(&self) -> dyn ExecutorView;
}

/// Adapter to convert a `StateView` into an `ExecutorView`.
pub struct ExecutorViewBase<'s, S> {
    base: &'s S,
}

impl<'s, S: StateView> ExecutorViewBase<'s, S> {
    pub(crate) fn new(base: &'s S) -> Self {
        Self {
            base,
        }
    }
}

impl<S: StateView> AsExecutorView<S> for S {
    fn as_executor_view(&self) -> ExecutorViewBase<S> {
        ExecutorViewBase::new(self)
    }
}

impl<'s, S: StateView> TAggregatorV1View for ExecutorViewBase<'s, S> {
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(
        &self,
        state_key: &Self::Identifier,
        // Reading from StateView can be in precise mode only.
    ) -> anyhow::Result<Option<StateValue>> {
        self.base.get_state_value(state_key)
    }
}

impl<'s, S: StateView> TDelayedFieldView for ExecutorViewBase<'s, S> {
    type Identifier = DelayedFieldID;

    fn is_aggregator_v2_delayed_fields_enabled(&self) -> bool {
        false
    }

    fn generate_delayed_field_id(&self) -> Self::Identifier {
        unimplemented!()
    }

    fn validate_and_convert_delayed_field_id(&self, _id: u64) -> Result<Self::Identifier, PanicError> {
        unimplemented!()
    }

    fn get_delayed_field_value(
        &self,
        _id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        unimplemented!()
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        _id: &Self::Identifier,
        _base_delta: &SignedU128,
        _delta: &SignedU128,
        _max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
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
