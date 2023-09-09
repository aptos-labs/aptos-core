// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::AggregatorResolver;
use aptos_table_natives::TableResolver;
use aptos_types::{
    on_chain_config::ConfigStorage,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
};
use aptos_vm_types::resolver::{ExecutorResolver, StateStorageResolver};
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, resolver::MoveResolver,
};
use std::collections::BTreeMap;

pub type StateValueMetadataKind = Option<StateValueMetadata>;
pub enum StateValueKind {
    Code,
    Data,
}

/// Allows to query storage metadata in the VM session. Needed for storage refunds.
pub trait StateValueMetadataResolver {
    /// Returns metadata for a given state value:
    ///   - None             if state value does not exist,
    ///   - Some(None)       if state value has no metadata,
    ///   - Some(Some(..))   otherwise.
    fn get_state_value_metadata(
        &self,
        state_key: &StateKey,
        // Allows to avoid deserialization of the access path from the state key.
        kind: StateValueKind,
    ) -> anyhow::Result<Option<StateValueMetadataKind>>;
}

/// A general resolver used by AptosVM. Allows to implement custom hooks on
/// top of storage, e.g. get resources from resource groups, etc.
pub trait AptosMoveResolver:
    AggregatorResolver
    + ConfigStorage
    + MoveResolver
    + TableResolver
    + StateValueMetadataResolver
    + StateStorageResolver
    + AsExecutorResolver
{
    fn release_resource_group_cache(
        &self,
    ) -> BTreeMap<AccountAddress, BTreeMap<StructTag, BTreeMap<StructTag, Vec<u8>>>>;
}

pub trait AsExecutorResolver {
    fn as_executor_resolver(&self) -> &dyn ExecutorResolver;
}
