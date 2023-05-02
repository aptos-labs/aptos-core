// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::{natives::state_storage::StateStorageUsageResolver, RuntimeModuleMetadataV1};
use aptos_state_view::StateView;
use aptos_types::on_chain_config::ConfigStorage;
use move_binary_format::errors::{Location, PartialVMError, VMError};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    resolver::MoveResolver,
    vm_status::StatusCode,
};
use move_table_extension::TableResolver;
use std::collections::BTreeMap;

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

    fn get_any_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError> {
        let metadata = self.get_module_metadata(struct_tag.module_id());
        let resource_group = Self::get_resource_group_from_metadata(struct_tag, metadata);
        if let Some(resource_group) = resource_group {
            self.get_resource_from_group(address, struct_tag, &resource_group)
        } else {
            self.get_standard_resource(address, struct_tag)
        }
    }

    fn get_resource_from_group(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        resource_group: &StructTag,
    ) -> Result<Option<Vec<u8>>, VMError> {
        let group_data = self.get_resource_group_data(address, resource_group)?;
        if let Some(group_data) = group_data {
            let mut group_data: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(&group_data)
                .map_err(|_| {
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .finish(Location::Undefined)
                })?;
            Ok(group_data.remove(struct_tag))
        } else {
            Ok(None)
        }
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
