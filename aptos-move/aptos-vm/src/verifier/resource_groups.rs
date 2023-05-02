// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::SessionExt;
use aptos_framework::{ResourceGroupScope, RuntimeModuleMetadataV1};
use move_binary_format::{
    errors::{Location, PartialVMError, VMError, VMResult},
    CompiledModule,
};
use move_core_types::{
    language_storage::{ModuleId, StructTag},
    vm_status::StatusCode,
};
use std::collections::BTreeMap;

fn metadata_validation_err(msg: &str) -> Result<(), VMError> {
    Err(metadata_validation_error(msg))
}

fn metadata_validation_error(msg: &str) -> VMError {
    PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
        .with_message(format!("metadata and code bundle mismatch: {}", msg))
        .finish(Location::Undefined)
}

/// Perform validation and upgrade checks on resource groups
/// * Acquire all relevant pieces of metadata
/// * Verify that there are no duplicate attributes.
/// * Ensure that each member has a membership and it does not change
/// * Ensure that each group has a scope and that it does not become more restrictive
/// * For any new members, verify that they are in a valid resource group
pub(crate) fn validate_resource_groups(
    session: &mut SessionExt,
    modules: &[CompiledModule],
) -> Result<(), VMError> {
    let mut groups = BTreeMap::new();
    let mut members = BTreeMap::new();

    for module in modules {
        let (new_groups, new_members) = validate_module_and_extract_new_entries(session, module)?;
        groups.insert(module.self_id(), new_groups);
        members.insert(module.self_id(), new_members);
    }

    for (module_id, inner_members) in members {
        for value in inner_members.values() {
            let value_module_id = value.module_id();
            if !groups.contains_key(&value_module_id) {
                let (inner_groups, _) =
                    extract_resource_group_metadata_from_module(session, &value_module_id)?;
                groups.insert(value.module_id(), inner_groups);
            }

            let scope = if let Some(inner_group) = groups.get(&value_module_id) {
                inner_group
                    .get(value.name.as_ident_str().as_str())
                    .ok_or_else(|| metadata_validation_error("Invalid resource_group attribute"))?
            } else {
                return Err(metadata_validation_error("No such resource_group"));
            };

            if !scope.are_equal_module_ids(&module_id, &value_module_id) {
                metadata_validation_err("Scope mismatch")?;
            }
        }
    }

    Ok(())
}

/// Validate resource group metadata on a single module
/// * Extract the resource group metadata
/// * Verify all changes are compatible upgrades
/// * Return any new members to validate correctness and all groups to assist in validation
pub(crate) fn validate_module_and_extract_new_entries(
    session: &mut SessionExt,
    module: &CompiledModule,
) -> VMResult<(
    BTreeMap<String, ResourceGroupScope>,
    BTreeMap<String, StructTag>,
)> {
    let (new_groups, mut new_members) =
        if let Some(metadata) = aptos_framework::get_metadata_from_compiled_module(module) {
            extract_resource_group_metadata(&metadata)?
        } else {
            (BTreeMap::new(), BTreeMap::new())
        };

    let (original_groups, original_members) =
        extract_resource_group_metadata_from_module(session, &module.self_id())?;

    for (member, value) in original_members {
        // We don't need to re-validate new_members above.
        if Some(&value) != new_members.remove(&member).as_ref() {
            metadata_validation_err("Invalid change in resource_group_member")?;
        }
    }

    for (group, value) in original_groups {
        // We need groups in case there's cross module dependencies
        if let Some(new_value) = new_groups.get(&group) {
            if value.is_less_strict(new_value) {
                metadata_validation_err("Invalid change in resource_group")?;
            }
        } else {
            metadata_validation_err("Invalid change in resource_group")?;
        }
    }

    Ok((new_groups, new_members))
}

/// Given a module id extract all resource group metadata
pub(crate) fn extract_resource_group_metadata_from_module(
    session: &mut SessionExt,
    module_id: &ModuleId,
) -> VMResult<(
    BTreeMap<String, ResourceGroupScope>,
    BTreeMap<String, StructTag>,
)> {
    let metadata = session.load_module(module_id).map(|module| {
        CompiledModule::deserialize(&module)
            .map(|module| aptos_framework::get_metadata_from_compiled_module(&module))
    });

    if let Ok(Ok(Some(metadata))) = metadata {
        extract_resource_group_metadata(&metadata)
    } else {
        Ok((BTreeMap::new(), BTreeMap::new()))
    }
}

/// Given a module id extract all resource group metadata
pub(crate) fn extract_resource_group_metadata(
    metadata: &RuntimeModuleMetadataV1,
) -> VMResult<(
    BTreeMap<String, ResourceGroupScope>,
    BTreeMap<String, StructTag>,
)> {
    let mut groups = BTreeMap::new();
    let mut members = BTreeMap::new();
    for (struct_, attrs) in &metadata.struct_attributes {
        for attr in attrs {
            if attr.is_resource_group() {
                let group = attr
                    .get_resource_group()
                    .ok_or_else(|| metadata_validation_error("Invalid resource_group attribute"))?;
                let old = groups.insert(struct_.clone(), group);
                if old.is_some() {
                    metadata_validation_err("Found duplicate resource_group attribute")?;
                }
            } else if attr.is_resource_group_member() {
                let member = attr.get_resource_group_member().ok_or_else(|| {
                    metadata_validation_error("Invalid resource_group_member attribute")
                })?;
                let old = members.insert(struct_.clone(), member);
                if old.is_some() {
                    metadata_validation_err("Found duplicate resource_group_member attribute")?;
                }
            }
        }
    }
    Ok((groups, members))
}
