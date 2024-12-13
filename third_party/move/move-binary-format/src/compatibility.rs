// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access::ModuleAccess,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        AbilitySet, Signature, SignatureToken, StructHandleIndex, StructTypeParameter,
        VariantIndex, Visibility,
    },
    file_format_common::VERSION_5,
    views::{
        FieldDefinitionView, ModuleView, StructDefinitionView, StructHandleView, ViewInternals,
    },
    CompiledModule,
};
use move_core_types::vm_status::StatusCode;
use std::collections::BTreeSet;

/// The result of a linking and layout compatibility check. Here is what the different combinations. NOTE that if `check_struct_layout` is false, type safety over a series of upgrades cannot be guaranteed.
/// mean:
/// `{ check_struct_and_pub_function_linking: true, check_struct_layout: true, check_friend_linking: true }`: fully backward compatible
/// `{ check_struct_and_pub_function_linking: true, check_struct_layout: true, check_friend_linking: false }`: Backward compatible, exclude the friend module declare and friend functions
/// `{ check_struct_and_pub_function_linking: false, check_struct_layout: true, check_friend_linking: false }`: Dependent modules that reference functions or types in this module may not link. However, fixing, recompiling, and redeploying all dependent modules will work--no data migration needed.
/// `{ check_struct_and_pub_function_linking: true, check_struct_layout: false, check_friend_linking: true }`: Attempting to read structs published by this module will now fail at runtime. However, dependent modules will continue to link. Requires data migration, but no changes to dependent modules.
/// `{ check_struct_and_pub_function_linking: false, check_struct_layout: false, check_friend_linking: false }`: Everything is broken. Need both a data migration and changes to dependent modules.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct Compatibility {
    /// if false, do not ensure the dependent modules that reference public functions or structs in this module can link
    pub(crate) check_struct_and_pub_function_linking: bool,
    /// if false, do not ensure the struct layout capability
    pub(crate) check_struct_layout: bool,
    /// if false, treat `friend` as `private` when `check_struct_and_function_linking`.
    pub(crate) check_friend_linking: bool,
    /// if false, entry function will be treated as regular function.
    pub(crate) treat_entry_as_public: bool,
}

impl Default for Compatibility {
    fn default() -> Self {
        Self {
            check_struct_and_pub_function_linking: true,
            check_struct_layout: true,
            check_friend_linking: true,
            treat_entry_as_public: true,
        }
    }
}

impl Compatibility {
    pub fn full_check() -> Self {
        Self::default()
    }

    pub fn no_check() -> Self {
        Self {
            check_struct_and_pub_function_linking: false,
            check_struct_layout: false,
            check_friend_linking: false,
            treat_entry_as_public: false,
        }
    }

    pub fn new(
        check_struct_layout: bool,
        check_friend_linking: bool,
        treat_entry_as_public: bool,
    ) -> Self {
        Self {
            check_struct_and_pub_function_linking: true,
            check_struct_layout,
            check_friend_linking,
            treat_entry_as_public,
        }
    }

    pub fn need_check_compat(&self) -> bool {
        self.check_struct_and_pub_function_linking
            || self.check_friend_linking
            || self.check_struct_layout
    }

    /// Check compatibility for `new_module` relative to old module `old_module`.
    pub fn check(
        &self,
        old_module: &CompiledModule,
        new_module: &CompiledModule,
    ) -> PartialVMResult<()> {
        let mut errors = vec![];

        // module's name and address are unchanged
        if old_module.address() != new_module.address() {
            errors.push(format!(
                "module address changed to `{}`",
                new_module.address()
            ));
        }
        if old_module.name() != new_module.name() {
            errors.push(format!("module name changed to `{}`", new_module.name()));
        }

        let old_view = ModuleView::new(old_module);
        let new_view = ModuleView::new(new_module);

        // old module's structs are a subset of the new module's structs
        for old_struct in old_view.structs() {
            let new_struct = match new_view.struct_definition(old_struct.name()) {
                Some(new_struct) => new_struct,
                None => {
                    // Struct not present in new . Existing modules that depend on this struct will fail to link with the new version of the module.
                    // Also, struct layout cannot be guaranteed transitively, because after
                    // removing the struct, it could be re-added later with a different layout.
                    errors.push(format!("removed struct `{}`", old_struct.name()));
                    break;
                },
            };

            if !struct_abilities_compatible(old_struct.abilities(), new_struct.abilities()) {
                errors.push(format!(
                    "removed abilities `{}` from struct `{}`",
                    old_struct.abilities().setminus(new_struct.abilities()),
                    old_struct.name()
                ));
            }
            if !struct_type_parameters_compatible(
                old_struct.type_parameters(),
                new_struct.type_parameters(),
            ) {
                errors.push(format!(
                    "changed type parameters of struct `{}`",
                    old_struct.name()
                ));
            }
            // Layout of old and new struct need to be compatible
            if self.check_struct_layout && !struct_layout_compatible(&old_struct, new_struct) {
                errors.push(format!("changed layout of struct `{}`", old_struct.name()));
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
        for old_func in old_view.functions() {
            // private, non entry function doesn't need to follow any checks here, skip
            if old_func.visibility() == Visibility::Private && !old_func.is_entry() {
                // Function not exposed, continue with next one
                continue;
            }
            let new_func = match new_view.function_definition(old_func.name()) {
                Some(new_func) => new_func,
                None => {
                    // Function has been removed
                    // Function is NOT a private, non entry function.
                    if !matches!(old_func.visibility(), Visibility::Friend)
                        // Above: Either Private Entry, or Public
                        || self.check_friend_linking
                        // Here we know that the old_function has to be Friend.
                        // And if friends are not considered private (self.check_friend_linking is true), we can't update.
                        || (old_func.is_entry() && self.treat_entry_as_public)
                    // Here we know that the old_func has to be Friend, and the check_friend_linking is set to false.
                    // We make sure that we don't allow any Entry functions to be deleted, when self.treat_entry_as_public is set (treats entry as public)
                    {
                        errors.push(format!("removed function `{}`", old_func.name()));
                    }
                    continue;
                },
            };

            if matches!(old_func.visibility(), Visibility::Friend)
                && !self.check_friend_linking
                // Above: We want to skip linking checks for public(friend) if self.check_friend_linking is set to false.
                && !(old_func.is_entry() && self.treat_entry_as_public)
            // However, public(friend) entry function still needs to be checked.
            {
                continue;
            }
            let is_vis_compatible = match (old_func.visibility(), new_func.visibility()) {
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
            let is_entry_compatible =
                if old_view.module().version < VERSION_5 && new_view.module().version < VERSION_5 {
                    // if it was public(script), it must remain public(script)
                    // if it was not public(script), it _cannot_ become public(script)
                    old_func.is_entry() == new_func.is_entry()
                } else {
                    // If it was an entry function, it must remain one.
                    // If it was not an entry function, it is allowed to become one.
                    !old_func.is_entry() || new_func.is_entry()
                };
            let error_msg = if !is_vis_compatible {
                Some("changed visibility")
            } else if !is_entry_compatible {
                Some("removed `entry` modifier")
            } else if !signature_compatible(
                old_module,
                old_func.parameters(),
                new_module,
                new_func.parameters(),
            ) {
                Some("changed parameter types")
            } else if !signature_compatible(
                old_module,
                old_func.return_type(),
                new_module,
                new_func.return_type(),
            ) {
                Some("changed return type")
            } else if !fun_type_parameters_compatible(
                old_func.type_parameters(),
                new_func.type_parameters(),
            ) {
                Some("changed type parameters")
            } else {
                None
            };
            if let Some(msg) = error_msg {
                errors.push(format!("{} of function `{}`", msg, old_func.name()));
            }
        }

        // check friend declarations compatibility
        //
        // - additions to the list are allowed
        // - removals are not allowed
        //
        if self.check_friend_linking {
            let old_friend_module_ids: BTreeSet<_> =
                old_module.immediate_friends().iter().cloned().collect();
            let new_friend_module_ids: BTreeSet<_> =
                new_module.immediate_friends().iter().cloned().collect();
            if !old_friend_module_ids.is_subset(&new_friend_module_ids) {
                errors.push(format!(
                    "removed friend declaration {}",
                    old_friend_module_ids
                        .difference(&new_friend_module_ids)
                        .map(|id| format!("`{}`", id))
                        .collect::<Vec<_>>()
                        .join(" and ")
                ))
            }
        }

        if !errors.is_empty() {
            Err(
                PartialVMError::new(StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE).with_message(
                    format!(
                        "Module update failure: new module not compatible with \
                        existing module in `{}`: {}",
                        old_view.id(),
                        errors.join(", ")
                    ),
                ),
            )
        } else {
            Ok(())
        }
    }
}

// When upgrading, the new abilities must be a superset of the old abilities.
// Adding an ability is fine, but removing an ability could cause existing usages to fail.
fn struct_abilities_compatible(old_abilities: AbilitySet, new_abilities: AbilitySet) -> bool {
    old_abilities.is_subset(new_abilities)
}

// When upgrading, the new type parameters must be the same length, and the new type parameter
// constraints must be compatible
fn fun_type_parameters_compatible(
    old_type_parameters: &[AbilitySet],
    new_type_parameters: &[AbilitySet],
) -> bool {
    old_type_parameters.len() == new_type_parameters.len()
        && old_type_parameters.iter().zip(new_type_parameters).all(
            |(old_type_parameter_constraint, new_type_parameter_constraint)| {
                type_parameter_constraints_compatible(
                    *old_type_parameter_constraint,
                    *new_type_parameter_constraint,
                )
            },
        )
}

fn struct_type_parameters_compatible(
    old_type_parameters: &[StructTypeParameter],
    new_type_parameters: &[StructTypeParameter],
) -> bool {
    old_type_parameters.len() == new_type_parameters.len()
        && old_type_parameters.iter().zip(new_type_parameters).all(
            |(old_type_parameter, new_type_parameter)| {
                type_parameter_phantom_decl_compatible(old_type_parameter, new_type_parameter)
                    && type_parameter_constraints_compatible(
                        old_type_parameter.constraints,
                        new_type_parameter.constraints,
                    )
            },
        )
}

fn struct_layout_compatible(
    old_struct: &StructDefinitionView<CompiledModule>,
    new_struct: &StructDefinitionView<CompiledModule>,
) -> bool {
    if old_struct.variant_count() == 0 {
        // Old is regular struct, new needs to be as well (i.e. have zero variants) and compatible
        // fields.
        new_struct.variant_count() == 0
            && fields_compatible(
                old_struct.fields_optional_variant(None),
                new_struct.fields_optional_variant(None),
            )
    } else {
        // Enum: the prefix of variants in the old definition must be the same as in the new one.
        // (a) the variant names need to match
        // (b) the variant fields need to be compatible
        old_struct.variant_count() <= new_struct.variant_count()
            && (0..old_struct.variant_count()).all(|i| {
                let v_idx = i as VariantIndex;
                old_struct.variant_name(v_idx) == new_struct.variant_name(v_idx)
                    && fields_compatible(
                        old_struct.fields_optional_variant(Some(v_idx)),
                        new_struct.fields_optional_variant(Some(v_idx)),
                    )
            })
    }
}

fn fields_compatible<'a, 'b>(
    mut old_fields: impl Iterator<Item = FieldDefinitionView<'a, CompiledModule>>,
    mut new_fields: impl Iterator<Item = FieldDefinitionView<'b, CompiledModule>>,
) -> bool {
    loop {
        match (old_fields.next(), new_fields.next()) {
            (Some(old_field), Some(new_field)) => {
                // Require names and types to be equal. Notice this is a stricter definition
                // than required. We could in principle choose that changing the name
                // (but not position or type) of a field is compatible. The VM does not care about
                // the name of a field but clients presumably do.
                if old_field.name() != new_field.name()
                    || !signature_token_compatible(
                        old_field.module(),
                        old_field.signature_token(),
                        new_field.module(),
                        new_field.signature_token(),
                    )
                {
                    return false;
                }
            },
            (None, None) => return true,
            _ => return false,
        }
    }
}

fn signature_compatible(
    old_module: &CompiledModule,
    old_sig: &Signature,
    new_module: &CompiledModule,
    new_sig: &Signature,
) -> bool {
    old_sig.0.len() == new_sig.0.len()
        && old_sig
            .0
            .iter()
            .zip(new_sig.0.iter())
            .all(|(old_tok, new_tok)| {
                signature_token_compatible(old_module, old_tok, new_module, new_tok)
            })
}

fn signature_token_compatible(
    old_module: &CompiledModule,
    old_tok: &SignatureToken,
    new_module: &CompiledModule,
    new_tok: &SignatureToken,
) -> bool {
    match (old_tok, new_tok) {
        (SignatureToken::Reference(old_elem), SignatureToken::Reference(new_elem)) => {
            signature_token_compatible(old_module, old_elem, new_module, new_elem)
        },
        (
            SignatureToken::MutableReference(old_elem),
            SignatureToken::MutableReference(new_elem),
        ) => signature_token_compatible(old_module, old_elem, new_module, new_elem),
        (SignatureToken::Vector(old_elem), SignatureToken::Vector(new_elem)) => {
            signature_token_compatible(old_module, old_elem, new_module, new_elem)
        },
        (SignatureToken::Struct(old_handle), SignatureToken::Struct(new_handle)) => {
            struct_equal(old_module, *old_handle, new_module, *new_handle)
        },
        (
            SignatureToken::StructInstantiation(old_handle, old_args),
            SignatureToken::StructInstantiation(new_handle, new_args),
        ) => {
            struct_equal(old_module, *old_handle, new_module, *new_handle)
                && old_args.len() == new_args.len()
                && (0..old_args.len()).all(|i| {
                    signature_token_compatible(old_module, &old_args[i], new_module, &new_args[i])
                })
        },
        _ => {
            // Map to representation equality. Notice that after handling of the cases above,
            // two signature tokens which have equal representation are also logically equal.
            // This is _not_ the case if handles are involved: for instance, Struct(h1) and
            // Struct(h2) can be different even if h1 == h2, but stem from different modules.
            old_tok == new_tok
        },
    }
}

fn struct_equal(
    old_module: &CompiledModule,
    old_handle: StructHandleIndex,
    new_module: &CompiledModule,
    new_handle: StructHandleIndex,
) -> bool {
    let old_struct = StructHandleView::new(old_module, old_module.struct_handle_at(old_handle));
    let new_struct = StructHandleView::new(new_module, new_module.struct_handle_at(new_handle));
    old_struct.name() == new_struct.name() && old_struct.module_id() == new_struct.module_id()
}

// When upgrading, the new constraints must be a subset of (or equal to) the old constraints.
// Removing an ability is fine, but adding an ability could cause existing callsites to fail
fn type_parameter_constraints_compatible(
    old_type_constraints: AbilitySet,
    new_type_constraints: AbilitySet,
) -> bool {
    new_type_constraints.is_subset(old_type_constraints)
}

// Adding a phantom annotation to a parameter won't break clients because that can only increase the
// the set of abilities in struct instantiations. Put it differently, adding phantom declarations
// relaxes the requirements for clients.
fn type_parameter_phantom_decl_compatible(
    old_type_parameter: &StructTypeParameter,
    new_type_parameter: &StructTypeParameter,
) -> bool {
    // old_type_paramter.is_phantom => new_type_parameter.is_phantom
    !old_type_parameter.is_phantom || new_type_parameter.is_phantom
}
