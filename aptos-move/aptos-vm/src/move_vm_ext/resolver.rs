// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::{AggregatorV1Resolver, DelayedFieldResolver};
use aptos_table_natives::TableResolver;
use aptos_types::{on_chain_config::ConfigStorage, state_store::state_key::StateKey};
use aptos_vm_types::resolver::{
    ExecutorView, ResourceGroupView, StateStorageView, StateValueMetadataResolver,
};
use bytes::Bytes;
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use std::collections::{BTreeMap, HashMap};

/// A general resolver used by AptosVM. Allows to implement custom hooks on
/// top of storage, e.g. get resources from resource groups, etc.
/// MoveResolver implements ResourceResolver and ModuleResolver
pub trait AptosMoveResolver:
    AggregatorV1Resolver
    + ConfigStorage
    + DelayedFieldResolver
    + MoveResolver
    + ResourceGroupResolver
    + StateValueMetadataResolver
    + StateStorageView
    + TableResolver
    + AsExecutorView
    + AsResourceGroupView
{
}

pub trait ResourceGroupResolver {
    /// If the option is Some, then it contains contents of the resource group
    /// cache with all resources in it, and the VM must use this to prepare the
    /// output as a combined group write as a part of normal resource writes (V0
    /// behavior). When None is returned, the V1 resource group behavior must be
    /// triggered, preparing more granular GroupWrite output (with individual
    /// writes to affected resources within the group). If the bool is true, then
    /// the gas should be charged in the V1 / granular fashion (AsSum), even if
    /// the output is being prepared in the V0 way (i.e. Option is set / Some).
    fn release_resource_group_cache(
        &self,
    ) -> (Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>>, bool);

    fn resource_group_size(&self, group_key: &StateKey) -> anyhow::Result<u64>;

    fn resource_size_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> anyhow::Result<u64>;

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
