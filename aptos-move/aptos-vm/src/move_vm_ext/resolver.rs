// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{natives::state_storage::StateStorageUsageResolver, RuntimeModuleMetadataV1};
use aptos_types::on_chain_config::ConfigStorage;
use aptos_vm_types::remote_cache::StateViewWithRemoteCache;
use move_binary_format::errors::VMError;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
};
use move_table_extension::TableResolver;
use move_vm_types::resolver::{MoveRefResolver, ResourceRef};

pub trait MoveResolverExt:
    MoveRefResolver<Err = VMError>
    + TableResolver
    + StateStorageUsageResolver
    + ConfigStorage
    + StateViewWithRemoteCache
{
    fn get_module_metadata(&self, module_id: ModuleId) -> Option<RuntimeModuleMetadataV1>;

    fn get_resource_group_data(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError>;

    fn get_standard_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError>;

    fn get_any_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError> {
        panic!("Cannot call 'get_any_resource' -- no support for resource groups")
    }

    fn get_resource_from_group(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        resource_group: &StructTag,
    ) -> Result<Option<ResourceRef>, VMError> {
        panic!("Cannot call 'get_resource_from_group', - resource groups are not supported yet!")
    }

    fn get_resource_group(&self, struct_tag: &StructTag) -> Result<Option<StructTag>, VMError> {
        let metadata = self.get_module_metadata(struct_tag.module_id());
        Ok(Self::get_resource_group_from_metadata(struct_tag, metadata))
    }

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

    fn is_resource_group(&self, struct_tag: &StructTag) -> bool {
        let metadata = self.get_module_metadata(struct_tag.module_id());
        metadata
            .and_then(|metadata| {
                metadata
                    .struct_attributes
                    .get(struct_tag.name.as_ident_str().as_str())
                    .and_then(|attrs| {
                        attrs
                            .iter()
                            .map(|attr| Some(attr.is_resource_group()))
                            .next()
                    })
            })
            .is_some()
    }
}
