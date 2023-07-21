// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::natives::state_storage::StateStorageUsageResolver;
use aptos_state_view::StateView;
use aptos_types::on_chain_config::ConfigStorage;
use aptos_utils::aptos_try;
use move_binary_format::errors::VMResult;
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, resolver::MoveResolver,
};
use move_table_extension::TableResolver;
use std::collections::BTreeMap;

pub trait MoveResolverExt:
    MoveResolver + TableResolver + StateStorageUsageResolver + ConfigStorage + StateView
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

    // Move to API does not belong here
    fn is_resource_group(&self, struct_tag: &StructTag) -> bool {
        aptos_try!({
            let md =
                aptos_framework::get_metadata(&self.get_module_metadata(&struct_tag.module_id()))?;
            md.struct_attributes
                .get(struct_tag.name.as_ident_str().as_str())?
                .iter()
                .find(|attr| attr.is_resource_group())?;
            Some(())
        })
        .is_some()
    }
}
