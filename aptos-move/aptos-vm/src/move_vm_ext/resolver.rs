// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::resolver::AggregatorResolver;
use aptos_framework::natives::state_storage::StateStorageUsageResolver;
use aptos_table_natives::TableResolver;
use aptos_types::on_chain_config::ConfigStorage;
use aptos_vm_types::view::StateView;
use move_binary_format::errors::VMResult;
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, resolver::MoveResolver,
};
use std::collections::BTreeMap;

pub trait AptosMoveResolver:
    MoveResolver + TableResolver + AggregatorResolver + StateStorageUsageResolver + ConfigStorage
{
    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> VMResult<Option<Vec<u8>>>;

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> VMResult<Option<Vec<u8>>>;

    fn release_resource_group_cache(
        &self,
    ) -> BTreeMap<AccountAddress, BTreeMap<StructTag, BTreeMap<StructTag, Vec<u8>>>>;
}

pub trait MoveResolverExt: AptosMoveResolver + StateView {}

impl<T: AptosMoveResolver + StateView> MoveResolverExt for T {}
