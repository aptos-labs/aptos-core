// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{natives::state_storage::StateStorageUsageResolver, RuntimeModuleMetadataV1};
use move_binary_format::errors::VMError;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    resolver::MoveResolver,
};
use move_table_extension::TableResolver;

pub trait MoveResolverExt:
    MoveResolver<Err = VMError> + TableResolver + StateStorageUsageResolver
{
    fn get_module_metadata(&self, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1>;

    fn get_resource_from_group(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        resource_group: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError>;

    fn get_resource_group(&self, struct_tag: &StructTag) -> Result<Option<StructTag>, VMError> {
        let metadata = self.get_module_metadata(struct_tag.module_id());
        Ok(Self::get_resource_group_from_metadata(struct_tag, metadata))
    }

    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError>;

    fn get_resource_group_from_metadata(
        struct_tag: &StructTag,
        metadata: Option<aptos_framework::RuntimeModuleMetadataV1>,
    ) -> Option<StructTag> {
        metadata.and_then(|metadata| {
            metadata
                .struct_attributes
                .get(struct_tag.name.as_ident_str().as_str())
                .and_then(|attrs| {
                    attrs
                        .iter()
                        .find_map(|attr| attr.get_resource_group_member())
                })
        })
    }
}
