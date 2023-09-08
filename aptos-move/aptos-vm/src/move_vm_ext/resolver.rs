// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::AggregatorResolver;
use aptos_table_natives::TableResolver;
use aptos_types::on_chain_config::ConfigStorage;
use aptos_vm_types::resolver::{ModuleResolver, ResourceResolver, StateStorageResolver};
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, resolver::MoveResolver,
};
use std::collections::BTreeMap;

/// A general resolver used by AptosVM. Allows to implement custom hooks on
/// top of storage, e.g. get resources from resource groups, etc.
pub trait AptosMoveResolver:
    AggregatorResolver
    + ConfigStorage
    + MoveResolver
    + TableResolver
    + StateStorageResolver
    + ModuleResolver
    + ResourceResolver
{
    fn release_resource_group_cache(
        &self,
    ) -> BTreeMap<AccountAddress, BTreeMap<StructTag, BTreeMap<StructTag, Vec<u8>>>>;
}
