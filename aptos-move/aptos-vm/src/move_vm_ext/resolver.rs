// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{natives::state_storage::StateStorageUsageResolver, RuntimeModuleMetadataV1};
use aptos_state_view::StateView;
use aptos_types::on_chain_config::ConfigStorage;
use aptos_utils::{aptos_try, return_on_failure};
use move_binary_format::errors::VMError;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    resolver::MoveResolver,
};
use move_table_extension::TableResolver;

fn get_resource_group_from_metadata(
    struct_tag: &StructTag,
    metadata: Option<aptos_framework::RuntimeModuleMetadataV1>,
) -> Option<StructTag> {
    metadata?
        .struct_attributes
        .get(struct_tag.name.as_ident_str().as_str())?
        .iter()
        .find_map(|attr| attr.get_resource_group_member())
}

pub trait MoveResolverExt:
    MoveResolver + TableResolver + StateStorageUsageResolver + ConfigStorage + StateView
{
    fn get_module_metadata(&self, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1>;

    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError>;

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError>;

    fn get_resource_group(&self, struct_tag: &StructTag) -> Option<StructTag> {
        let metadata = self.get_module_metadata(struct_tag.module_id());
        get_resource_group_from_metadata(struct_tag, metadata)
    }

    // Move to API does not belong here
    fn is_resource_group(&self, struct_tag: &StructTag) -> bool {
        aptos_try!({
            return_on_failure!(self
                .get_module_metadata(struct_tag.module_id())?
                .struct_attributes
                .get(struct_tag.name.as_ident_str().as_str())?
                .iter()
                .find(|attr| attr.is_resource_group()));
            Some(())
        })
        .is_some()
    }
}
