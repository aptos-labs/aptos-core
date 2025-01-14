// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(deprecated)]

use crate::{
    compatibility,
    errors::{PartialVMError, PartialVMResult},
    file_format::{StructTypeParameter, Visibility},
    file_format_common::VERSION_5,
    normalized::Module,
};
use compatibility::Compatibility;
use move_core_types::{ability::AbilitySet, vm_status::StatusCode};
use std::collections::BTreeSet;

impl Compatibility {
    /// Check compatibility for `new_module` relative to old module `old_module`.
    pub fn legacy_check(&self, old_module: &Module, new_module: &Module) -> PartialVMResult<()> {
        let mut struct_and_pub_function_linking = true;
        let mut struct_layout = true;
        let mut friend_linking = true;

        // module's name and address are unchanged
        if old_module.address != new_module.address || old_module.name != new_module.name {
            struct_and_pub_function_linking = false;
        }

        // old module's structs are a subset of the new module's structs
        for (name, old_struct) in &old_module.structs {
            let new_struct = match new_module.structs.get(name) {
                Some(new_struct) => new_struct,
                None => {
                    // Struct not present in new . Existing modules that depend on this struct will fail to link with the new version of the module.
                    // Also, struct layout cannot be guaranteed transitively, because after
                    // removing the struct, it could be re-added later with a different layout.
                    struct_and_pub_function_linking = false;
                    struct_layout = false;
                    break;
                },
            };

            if !struct_abilities_compatibile(old_struct.abilities, new_struct.abilities)
                || !struct_type_parameters_compatibile(
                    &old_struct.type_parameters,
                    &new_struct.type_parameters,
                )
            {
                struct_and_pub_function_linking = false;
            }
            if new_struct.fields != old_struct.fields {
                // TODO(#13806): implement struct variants
                // Fields changed. Code in this module will fail at runtime if it tries to
                // read a previously published struct value
                // TODO: this is a stricter definition than required. We could in principle
                // choose that changing the name (but not position or type) of a field is
                // compatible. The VM does not care about the name of a field
                // (it's purely informational), but clients presumably do.
                struct_layout = false
            }
        }

        // The modules are considered as compatible function-wise when all the conditions are met:
        //
        // - old module's public functions are a subset of the new module's public functions
        //   (i.e. we cannot remove or change public functions)
        // - old module's script functions are a subset of the new module's script functions
        //   (i.e. we cannot remove or change script functions)
        // - for any friend function that is removed or changed in the old module
        //   - if the function visibility is upgraded to public, it is OK
        //   - otherwise, it is considered as incompatible.
        //
        for (name, old_func) in &old_module.exposed_functions {
            let new_func = match new_module.exposed_functions.get(name) {
                Some(new_func) => new_func,
                None => {
                    if matches!(old_func.visibility, Visibility::Friend)
                        && !(old_func.is_entry && self.treat_entry_as_public)
                    // self.treat_entry_as_public is false: trying to remove friend
                    // self.treat_entry_as_public is true:  trying to remove Friend non-entry
                    {
                        // Report as friend linking error, which would be dismissed when
                        // self.check_friend_linking is set to false
                        friend_linking = false;
                    } else {
                        // Otherwise report as function linking error.
                        struct_and_pub_function_linking = false;
                    }
                    continue;
                },
            };
            let is_vis_compatible = match (old_func.visibility, new_func.visibility) {
                // public must remain public
                (Visibility::Public, Visibility::Public) => true,
                (Visibility::Public, _) => false,
                // friend can become public or remain friend
                (Visibility::Friend, Visibility::Public)
                | (Visibility::Friend, Visibility::Friend) => true,
                (Visibility::Friend, _) => false,
                // private can become public or friend, or stay private
                (Visibility::Private, _) => true,
            };
            let is_entry_compatible = if old_module.file_format_version < VERSION_5
                && new_module.file_format_version < VERSION_5
            {
                // if it was public(script), it must remain pubic(script)
                // if it was not public(script), it _cannot_ become public(script)
                old_func.is_entry == new_func.is_entry
            } else {
                // If it was an entry function, it must remain one.
                // If it was not an entry function, it is allowed to become one.
                !old_func.is_entry || new_func.is_entry
            };
            if !is_vis_compatible
                || !is_entry_compatible
                || old_func.parameters != new_func.parameters
                || old_func.return_ != new_func.return_
                || !fun_type_parameters_compatibile(
                    &old_func.type_parameters,
                    &new_func.type_parameters,
                )
            {
                if matches!(old_func.visibility, Visibility::Friend)
                    && (!old_func.is_entry || !self.treat_entry_as_public)
                // self.treat_entry_as_public is false: trying to change signature of a friend function
                // self.treat_entry_as_public is true:  trying to change signature of a friend non-entry function.
                {
                    // Report as friend linking error, which would be dismissed when
                    // self.check_friend_linking is set to false
                    friend_linking = false;
                } else {
                    // Otherwise report as function linking error.
                    struct_and_pub_function_linking = false;
                }
            }
        }

        // check friend declarations compatibility
        //
        // - additions to the list are allowed
        // - removals are not allowed
        //
        let old_friend_module_ids: BTreeSet<_> = old_module.friends.iter().cloned().collect();
        let new_friend_module_ids: BTreeSet<_> = new_module.friends.iter().cloned().collect();
        if !old_friend_module_ids.is_subset(&new_friend_module_ids) {
            friend_linking = false;
        }

        if self.check_struct_and_pub_function_linking && !struct_and_pub_function_linking {
            return Err(PartialVMError::new(
                StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE,
            ).with_message(format!("Module Update Failure: Public function/struct signature of new module differs from existing module in {:?}::{}", old_module.address, old_module.name)));
        }
        if self.check_struct_layout && !struct_layout {
            return Err(PartialVMError::new(
                StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE,
            ).with_message(format!("Module Update Failure: Struct layout of new module differs from existing modul in {:?}::{}", old_module.address, old_module.name)));
        }
        if self.check_friend_linking && !friend_linking {
            return Err(PartialVMError::new(
                StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE,
            ).with_message(format!("Module Update Failure: Friend signature of new module differs from existing module in {:?}::{}", old_module.address, old_module.name)));
        }

        Ok(())
    }
}

// When upgrading, the new abilities must be a superset of the old abilities.
// Adding an ability is fine, but removing an ability could cause existing usages to fail.
fn struct_abilities_compatibile(old_abilities: AbilitySet, new_abilities: AbilitySet) -> bool {
    old_abilities.is_subset(new_abilities)
}

// When upgrading, the new type parameters must be the same length, and the new type parameter
// constraints must be compatible
fn fun_type_parameters_compatibile(
    old_type_parameters: &[AbilitySet],
    new_type_parameters: &[AbilitySet],
) -> bool {
    old_type_parameters.len() == new_type_parameters.len()
        && old_type_parameters.iter().zip(new_type_parameters).all(
            |(old_type_parameter_constraint, new_type_parameter_constraint)| {
                type_parameter_constraints_compatibile(
                    *old_type_parameter_constraint,
                    *new_type_parameter_constraint,
                )
            },
        )
}

fn struct_type_parameters_compatibile(
    old_type_parameters: &[StructTypeParameter],
    new_type_parameters: &[StructTypeParameter],
) -> bool {
    old_type_parameters.len() == new_type_parameters.len()
        && old_type_parameters.iter().zip(new_type_parameters).all(
            |(old_type_parameter, new_type_parameter)| {
                type_parameter_phantom_decl_compatibile(old_type_parameter, new_type_parameter)
                    && type_parameter_constraints_compatibile(
                        old_type_parameter.constraints,
                        new_type_parameter.constraints,
                    )
            },
        )
}

// When upgrading, the new constraints must be a subset of (or equal to) the old constraints.
// Removing an ability is fine, but adding an ability could cause existing callsites to fail
fn type_parameter_constraints_compatibile(
    old_type_constraints: AbilitySet,
    new_type_constraints: AbilitySet,
) -> bool {
    new_type_constraints.is_subset(old_type_constraints)
}

// Adding a phantom annotation to a parameter won't break clients because that can only increase the
// the set of abilities in struct instantiations. Put it differently, adding phantom declarations
// relaxes the requirements for clients.
fn type_parameter_phantom_decl_compatibile(
    old_type_parameter: &StructTypeParameter,
    new_type_parameter: &StructTypeParameter,
) -> bool {
    // old_type_paramter.is_phantom => new_type_parameter.is_phantom
    !old_type_parameter.is_phantom || new_type_parameter.is_phantom
}
