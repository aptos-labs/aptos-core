// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::{AggregatorReadMode, TAggregatorView};
use aptos_state_view::{StateView, StateViewId};
use aptos_types::{
    aggregator::AggregatorID,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
};
use aptos_vm_types::{
    resolver::{StateStorageView, TModuleView, TResourceGroupView, TResourceView},
    resource_group_adapter::{GroupSizeKind, ResourceGroupAdapter},
};
use bytes::Bytes;
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use std::collections::{BTreeMap, HashMap};

/// Adapter to convert a `StateView` into an `ExecutorView`.
pub struct ExecutorViewBase<'s, S>(&'s S, ResourceGroupAdapter<'s>);

impl<'s, S: StateView> ExecutorViewBase<'s, S> {
    pub(crate) fn new(state_view: &'s S, group_size_kind: GroupSizeKind) -> Self {
        Self(
            state_view,
            ResourceGroupAdapter::from_state_view(state_view, group_size_kind),
        )
    }
}

/// Convenience trait to use StateView as ExecutorView. The group size computation is
/// disabled, since that is only required by StorageAdapter that creates and configures
/// its own adapters for resource groups, as needed.
pub trait AsExecutorView<S> {
    fn as_executor_view(&self) -> ExecutorViewBase<S>;
}

impl<S: StateView> AsExecutorView<S> for S {
    fn as_executor_view(&self) -> ExecutorViewBase<S> {
        ExecutorViewBase::new(self, GroupSizeKind::None)
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

impl<'s, S: StateView> TResourceGroupView for ExecutorViewBase<'s, S> {
    type Key = StateKey;
    type Layout = MoveTypeLayout;
    type Tag = StructTag;

    fn resource_group_size(&self, state_key: &Self::Key) -> anyhow::Result<u64> {
        self.1.resource_group_size(state_key)
    }

    fn get_resource_from_group(
        &self,
        state_key: &Self::Key,
        resource_tag: &Self::Tag,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<Bytes>> {
        self.1
            .get_resource_from_group(state_key, resource_tag, maybe_layout)
    }

    fn release_naive_group_cache(&self) -> Option<HashMap<Self::Key, BTreeMap<Self::Tag, Bytes>>> {
        self.1.release_naive_group_cache()
    }
}
