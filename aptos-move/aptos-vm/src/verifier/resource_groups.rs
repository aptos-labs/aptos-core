// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    on_chain_config::{FeatureFlag, Features},
    vm::module_metadata::{
        get_metadata_from_compiled_code, ResourceGroupScope, RuntimeModuleMetadataV1,
    },
};
use move_binary_format::{
    access::ModuleAccess,
    errors::{Location, PartialVMError, VMError, VMResult},
    CompiledModule,
};
use move_core_types::{gas_algebra::NumBytes, language_storage::StructTag, vm_status::StatusCode};
use move_vm_runtime::{module_traversal::TraversalContext, ModuleStorage};
use move_vm_types::gas::{DependencyKind, GasMeter};
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
    features: &Features,
    module_storage: &impl ModuleStorage,
    traversal_context: &mut TraversalContext,
    gas_meter: &mut impl GasMeter,
    new_modules: &[CompiledModule],
) -> Result<(), VMError> {
    let mut groups = BTreeMap::new();
    let mut members = BTreeMap::new();

    for new_module in new_modules {
        let (new_groups, new_members) = validate_module_and_extract_new_entries(
            module_storage,
            new_module,
            features,
            traversal_context,
        )?;
        groups.insert(new_module.self_id(), new_groups);
        members.insert(new_module.self_id(), new_members);
    }

    for (module_id, inner_members) in members {
        for group_tag in inner_members.values() {
            let group_module_id = group_tag.module_id();
            if !groups.contains_key(&group_module_id) {
                // Note: module must exist for the group member to refer to it! Also, we need to
                // charge gas because this module is not in a bundle.
                if features.is_lazy_loading_enabled()
                    && traversal_context.visit_if_not_special_module_id(&group_module_id)
                {
                    let size = module_storage.unmetered_get_existing_module_size(
                        group_module_id.address(),
                        group_module_id.name(),
                    )?;
                    gas_meter
                        .charge_dependency(
                            DependencyKind::Existing,
                            group_module_id.address(),
                            group_module_id.name(),
                            NumBytes::new(size as u64),
                        )
                        .map_err(|err| err.finish(Location::Undefined))?;
                }
                let old_module = module_storage.unmetered_get_existing_deserialized_module(
                    group_module_id.address(),
                    group_module_id.name(),
                )?;

                let (inner_groups, _, _) =
                    extract_resource_group_metadata_from_module(&old_module)?;
                groups.insert(group_tag.module_id(), inner_groups);
            }

            let scope = if let Some(inner_group) = groups.get(&group_module_id) {
                inner_group
                    .get(group_tag.name.as_ident_str().as_str())
                    .ok_or_else(|| metadata_validation_error("Invalid resource_group attribute"))?
            } else {
                return Err(metadata_validation_error("No such resource_group"));
            };

            if !scope.are_equal_module_ids(&module_id, &group_module_id) {
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
    module_storage: &impl ModuleStorage,
    new_module: &CompiledModule,
    features: &Features,
    traversal_context: &TraversalContext,
) -> VMResult<(
    BTreeMap<String, ResourceGroupScope>,
    BTreeMap<String, StructTag>,
)> {
    let (new_groups, mut new_members) =
        if let Some(metadata) = get_metadata_from_compiled_code(new_module) {
            extract_resource_group_metadata(&metadata)?
        } else {
            (BTreeMap::new(), BTreeMap::new())
        };

    // INVARIANT:
    //   No need to charge gas for this module access: below we fetch old version of the module
    //   (based on the new module's name). Old versions are all charged when processing the publish
    //   request.
    if features.is_lazy_loading_enabled() {
        traversal_context
            .check_is_special_or_visited(new_module.address(), new_module.name())
            .map_err(|err| err.finish(Location::Undefined))?;
    }

    let (original_groups, original_members, mut structs) = module_storage
        .unmetered_get_deserialized_module(new_module.address(), new_module.name())?
        .map_or_else(
            || Ok((BTreeMap::new(), BTreeMap::new(), BTreeSet::new())),
            |old_module| extract_resource_group_metadata_from_module(&old_module),
        )?;

    for (member, group) in original_members {
        // We don't need to re-validate new_members above.
        if Some(group) != new_members.remove(&member) {
            metadata_validation_err("Invalid removal of resource_group_member attribute")?;
        }

        // For this to fail is an invariant violation, it means we allow for arbitrary upgrades.
        structs.remove(&member);
    }

    for (group, scope) in original_groups {
        // We need groups in case there's cross module dependencies
        if let Some(new_scope) = new_groups.get(&group) {
            if scope.is_less_strict(new_scope) {
                metadata_validation_err("Invalid removal of resource_group attribute")?;
            }
        } else {
            metadata_validation_err("Invalid change in resource_group")?;
        }

        // For this to fail is an invariant violation, it means we allow for arbitrary upgrades.
        structs.remove(&group);
    }

    if !features.is_enabled(FeatureFlag::SAFER_RESOURCE_GROUPS) {
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
    old_module: &CompiledModule,
) -> VMResult<(
    BTreeMap<String, ResourceGroupScope>,
    BTreeMap<String, StructTag>,
    BTreeSet<String>,
)> {
    if let Some(metadata) = get_metadata_from_compiled_code(old_module) {
        let (groups, members) = extract_resource_group_metadata(&metadata)?;
        let structs = old_module
            .struct_defs()
            .iter()
            .map(|struct_def| {
                let struct_handle = old_module.struct_handle_at(struct_def.struct_handle);
                old_module.identifier_at(struct_handle.name).to_string()
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
