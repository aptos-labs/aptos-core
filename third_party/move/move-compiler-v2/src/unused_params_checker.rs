// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements an environment pipeline which checks for unused type parameters in struct definitions.

use codespan_reporting::diagnostic::Severity;
use move_model::{
    model::{GlobalEnv, StructEnv, TypeParameter},
    ty::Type,
};
use std::collections::BTreeSet;

/// Checks all modules in the given environment for
/// unused type parameters in struct definitions.
pub fn unused_params_checker(env: &GlobalEnv) {
    for module in env.get_modules() {
        if module.is_target() {
            for struct_env in module.get_structs() {
                check_unused_params(&struct_env);
            }
        }
    }
}

/// Checks for unused type parameters for the given struct, and reports errors if found.
fn check_unused_params(struct_env: &StructEnv) {
    let env = struct_env.module_env.env;
    let used_params_in_fields = used_type_parameters_in_fields(struct_env);
    for (i, TypeParameter(name, kind, loc)) in struct_env.get_type_parameters().iter().enumerate() {
        if !kind.is_phantom && !used_params_in_fields.contains(&(i as u16)) {
            let name = name.display(struct_env.symbol_pool());
            env.diag_with_labels(Severity::Warning, loc, "unused type parameter", vec![(
                loc.clone(),
                format!(
                    "Unused type parameter `{}`. Consider declaring it as phantom",
                    name
                ),
            )]);
        }
    }
}

/// Returns the indices of type parameters used in the fields of the given struct.
fn used_type_parameters_in_fields(struct_env: &StructEnv) -> BTreeSet<u16> {
    struct_env
        .get_fields()
        .flat_map(|field_env| used_type_parameters_in_ty(&field_env.get_type()))
        .collect()
}

/// Returns the indices of type parameters used in the given type.
fn used_type_parameters_in_ty(ty: &Type) -> BTreeSet<u16> {
    match ty {
        Type::Primitive(_) => BTreeSet::new(),
        Type::Tuple(tys) | Type::Struct(_, _, tys) => {
            tys.iter().flat_map(used_type_parameters_in_ty).collect()
        },
        Type::TypeParameter(i) => {
            let mut set = BTreeSet::new();
            set.insert(*i);
            set
        },
        Type::Fun(from, to) => {
            let mut used_in_from = used_type_parameters_in_ty(from);
            used_in_from.append(&mut used_type_parameters_in_ty(to));
            used_in_from
        },
        Type::Vector(ty) | Type::Reference(_, ty) => used_type_parameters_in_ty(ty),
        Type::TypeDomain(_) | Type::ResourceDomain(_, _, _) | Type::Error | Type::Var(_) => {
            unreachable!("unexpected type")
        },
    }
}
