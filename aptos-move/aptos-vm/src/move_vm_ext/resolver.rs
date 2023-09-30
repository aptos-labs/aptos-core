// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::AggregatorResolver;
use aptos_table_natives::TableResolver;
use aptos_types::{on_chain_config::ConfigStorage, state_store::state_key::StateKey};
use aptos_vm_types::resolver::{ExecutorView, StateStorageView, StateValueMetadataResolver};
use bytes::Bytes;
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use std::collections::{BTreeMap, HashMap};

/// A general resolver used by AptosVM. Allows to implement custom hooks on
/// top of storage, e.g. get resources from resource groups, etc.
/// MoveResolver implements ResourceResolver and ModuleResolver
pub trait AptosMoveResolver:
    AggregatorResolver
    + ConfigStorage
    + MoveResolver
    + TableResolver
    + StateValueMetadataResolver
    + StateStorageView
    + AsExecutorView
{
    fn release_resource_group_cache(&self)
        -> Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>>;
}

pub trait AsExecutorView {
    fn as_executor_view(&self) -> &dyn ExecutorView;
}
