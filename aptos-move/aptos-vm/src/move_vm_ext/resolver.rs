// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::AggregatorResolver;
use aptos_framework::natives::state_storage::StateStorageUsageResolver;
use aptos_state_view::StateView;
use aptos_table_natives::TableResolver;
use aptos_types::{on_chain_config::ConfigStorage, state_store::state_key::StateKey};
use aptos_vm_types::resolver::{ResourceGroupResolver, StateValueMetadataResolver};
use bytes::Bytes;
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use std::collections::{BTreeMap, HashMap};

/// A general resolver used by AptosVM. Allows to implement custom hooks on
/// top of storage, e.g. get resources from resource groups, etc.
pub trait AptosMoveResolver:
    MoveResolver
    + AggregatorResolver
    + TableResolver
    + StateStorageUsageResolver
    + StateValueMetadataResolver
    + ConfigStorage
    + ResourceGroupResolver
{
    fn release_resource_group_cache(&self) -> HashMap<StateKey, BTreeMap<StructTag, Bytes>>;
}

// TODO: Remove dependency on StateView.
pub trait MoveResolverExt: AptosMoveResolver + StateView {}
impl<T: AptosMoveResolver + StateView> MoveResolverExt for T {}
