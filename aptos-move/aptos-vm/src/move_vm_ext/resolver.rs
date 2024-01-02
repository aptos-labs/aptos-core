// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::{AggregatorV1Resolver, DelayedFieldResolver};
use aptos_table_natives::TableResolver;
use aptos_types::{on_chain_config::ConfigStorage, state_store::state_key::StateKey};
use aptos_vm_types::resolver::{
    ExecutorView, ResourceGroupSize, ResourceGroupView, StateStorageView,
    StateValueMetadataResolver,
};
use bytes::Bytes;
use move_binary_format::errors::PartialVMError;
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use std::collections::{BTreeMap, HashMap};

/// A general resolver used by AptosVM. Allows to implement custom hooks on
/// top of storage, e.g. get resources from resource groups, etc.
/// MoveResolver implements ResourceResolver and ModuleResolver
pub trait AptosMoveResolver:
    AggregatorV1Resolver
    + ConfigStorage
    + DelayedFieldResolver
    + MoveResolver<PartialVMError>
    + ResourceGroupResolver
    + StateValueMetadataResolver
    + StateStorageView
    + TableResolver
    + AsExecutorView
    + AsResourceGroupView
{
}

pub trait ResourceGroupResolver {
    fn release_resource_group_cache(&self)
        -> Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>>;

    fn resource_group_size(&self, group_key: &StateKey) -> anyhow::Result<ResourceGroupSize>;

    fn resource_size_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> anyhow::Result<usize>;

    fn resource_exists_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> anyhow::Result<bool>;
}

pub trait AsExecutorView {
    fn as_executor_view(&self) -> &dyn ExecutorView;
}

pub trait AsResourceGroupView {
    fn as_resource_group_view(&self) -> &dyn ResourceGroupView;
}
