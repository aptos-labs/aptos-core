// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::AggregatorResolver;
use aptos_framework::natives::state_storage::StateStorageUsageResolver;
use aptos_table_natives::TableResolver;
use aptos_types::{
    on_chain_config::ConfigStorage,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
};
use aptos_vm_types::resolver::{ModuleResolver, ResourceResolver, StateStorageResolver};
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, resolver::MoveResolver,
};
use std::collections::BTreeMap;

/// Allows to query storage metadata in the VM session. Needed for storage refunds.
pub trait StateValueMetadataResolver {
    /// Returns metadata for a given state value:
    ///   - None             if state value does not exist,
    ///   - Some(None)       if state value has no metadata,
    ///   - Some(Some(..))   otherwise.
    // TODO: Nested options are ugly, refactor.
    fn get_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<Option<StateValueMetadata>>>;
}

/// A general resolver used by AptosVM. Allows to implement custom hooks on
/// top of storage, e.g. get resources from resource groups, etc.
pub trait AptosMoveResolver:
    MoveResolver
    + AggregatorResolver
    + TableResolver
    + StateStorageUsageResolver
    + StateValueMetadataResolver
    + ConfigStorage
{
    fn release_resource_group_cache(
        &self,
    ) -> BTreeMap<AccountAddress, BTreeMap<StructTag, BTreeMap<StructTag, Vec<u8>>>>;
}

// TODO: Remove dependency on StateView.
pub trait MoveResolverExt:
    AptosMoveResolver + ResourceResolver + ModuleResolver + StateStorageResolver
{
}

impl<T: AptosMoveResolver + ResourceResolver + ModuleResolver + StateStorageResolver>
    MoveResolverExt for T
{
}
