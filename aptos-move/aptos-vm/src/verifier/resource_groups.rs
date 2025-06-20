// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::vm::module_metadata::{
    get_metadata_from_compiled_code, ResourceGroupScope, RuntimeModuleMetadataV1,
};
use aptos_vm_types::module_and_script_storage::module_storage::AptosModuleStorage;
use move_binary_format::{
    access::ModuleAccess,
    errors::{Location, PartialVMError, VMError, VMResult},
    CompiledModule,
};
use move_core_types::{
    language_storage::{ModuleId, StructTag},
    vm_status::StatusCode,
};
use std::collections::{BTreeMap, BTreeSet};

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
    module_storage: &impl AptosModuleStorage,
    modules: &[CompiledModule],
    safer_resource_groups: bool,
) -> Result<(), VMError> {
    let mut groups = BTreeMap::new();
    let mut members = BTreeMap::new();

    for module in modules {
        let (new_groups, new_members) =
            validate_module_and_extract_new_entries(module_storage, module, safer_resource_groups)?;
        groups.insert(module.self_id(), new_groups);
        members.insert(module.self_id(), new_members);
    }

    for (module_id, inner_members) in members {
        for value in inner_members.values() {
            let value_module_id = value.module_id();
            if !groups.contains_key(&value_module_id) {
                let (inner_groups, _, _) =
                    extract_resource_group_metadata_from_module(module_storage, &value_module_id)?;
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
    module_storage: &impl AptosModuleStorage,
    module: &CompiledModule,
    safer_resource_groups: bool,
) -> VMResult<(
    BTreeMap<String, ResourceGroupScope>,
    BTreeMap<String, StructTag>,
)> {
    let (new_groups, mut new_members) =
        if let Some(metadata) = get_metadata_from_compiled_code(module) {
            extract_resource_group_metadata(&metadata)?
        } else {
            (BTreeMap::new(), BTreeMap::new())
        };

    let (original_groups, original_members, mut structs) =
        extract_resource_group_metadata_from_module(module_storage, &module.self_id())?;

    for (member, value) in original_members {
        // We don't need to re-validate new_members above.
        if Some(&value) != new_members.remove(&member).as_ref() {
            metadata_validation_err("Invalid removal of resource_group_member attribute")?;
        }

        // For this to fail is an invariant violation, it means we allow for arbitrary upgrades.
        structs.remove(&member);
    }

    for (group, value) in original_groups {
        // We need groups in case there's cross module dependencies
        if let Some(new_value) = new_groups.get(&group) {
            if value.is_less_strict(new_value) {
                metadata_validation_err("Invalid removal of resource_group attribute")?;
            }
        } else {
            metadata_validation_err("Invalid change in resource_group")?;
        }

        // For this to fail is an invariant violation, it means we allow for arbitrary upgrades.
        structs.remove(&group);
    }

    if !safer_resource_groups {
        return Ok((new_groups, new_members));
    }

    // At this point, only original structs that do not have resource group affiliation are left.
    // Note, we do not validate for being both a member and a group, because there are other
    // checks earlier on, such as, a resource group must have no abilities, while a resource group
    // member must.

    for group in new_groups.keys() {
        if structs.remove(group) {
            metadata_validation_err("Invalid addition of resource_group attribute")?;
        }
    }

    for member in new_members.keys() {
        if structs.remove(member) {
            metadata_validation_err("Invalid addition of resource_group_member attribute")?;
        }
    }

    Ok((new_groups, new_members))
}

/// Given a module id extract all resource group metadata
pub(crate) fn extract_resource_group_metadata_from_module(
    module_storage: &impl AptosModuleStorage,
    module_id: &ModuleId,
) -> VMResult<(
    BTreeMap<String, ResourceGroupScope>,
    BTreeMap<String, StructTag>,
    BTreeSet<String>,
)> {
    let module =
        module_storage.fetch_existing_deserialized_module(module_id.address(), module_id.name());
    let (metadata, module) = if let Ok(module) = module {
        (get_metadata_from_compiled_code(module.as_ref()), module)
    } else {
        // Maintaining backwards compatibility with no validation of deserialization.
        return Ok((BTreeMap::new(), BTreeMap::new(), BTreeSet::new()));
    };

    if let Some(metadata) = metadata {
        let (groups, members) = extract_resource_group_metadata(&metadata)?;
        let structs = module
            .struct_defs()
            .iter()
            .map(|struct_def| {
                let struct_handle = module.struct_handle_at(struct_def.struct_handle);
                let name = module.identifier_at(struct_handle.name).to_string();
                name
            })
            .collect::<BTreeSet<_>>();
        Ok((groups, members, structs))
    } else {
        Ok((BTreeMap::new(), BTreeMap::new(), BTreeSet::new()))
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
