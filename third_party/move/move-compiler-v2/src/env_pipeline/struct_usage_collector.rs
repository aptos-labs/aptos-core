// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! The struct usage collector runs early in the pipeline, before any transformations,
//! and populates the `users` field on `StructData` for each struct in the move model.
//!
//! The collector gathers struct usage from the following sources:
//!
//! - Function bodies, parameters, and return types
//! - Struct field types
//! - Note: specs are excluded
//!

use move_model::{
    ast::ResourceSpecifier,
    model::{GlobalEnv, QualifiedId, StructId, UserId},
    ty::Type,
};
use std::collections::BTreeSet;

pub fn collect_struct_usage(env: &mut GlobalEnv) {
    // We collect all usage into a BTreeSet first, then apply it at the end.
    // This is necessary because iterating over modules/functions/structs borrows `env`
    // immutably, but `env.add_struct_user()` requires a mutable borrow. Rust's borrow
    // checker doesn't allow both simultaneously.
    let mut usage = BTreeSet::new();

    for module in env.get_modules() {
        // Collect from functions
        for func in module.get_functions() {
            let func_qid = func.get_qualified_id();
            let func_loc = func.get_loc();

            // From function body (exclude specs)
            if let Some(def) = func.get_def() {
                for sid in def.struct_usage(env, false) {
                    // Use the expression-level loc for body usages when available;
                    // fall back to function loc.
                    usage.insert((sid, UserId::Function(func_qid, func_loc.clone())));
                }
            }

            // From function parameters and return type
            for param in func.get_parameters() {
                let user_id = UserId::Function(func_qid, func_loc.clone());
                collect_usage_from_type(&param.1, &user_id, &mut usage);
            }
            let user_id = UserId::Function(func_qid, func_loc);
            collect_usage_from_type(&func.get_result_type(), &user_id, &mut usage);

            // From access specifiers
            if let Some(specifiers) = func.get_access_specifiers() {
                for spec in specifiers {
                    if let ResourceSpecifier::Resource(qid) = &spec.resource.1 {
                        let user_id =
                            UserId::Function(func.get_qualified_id(), spec.resource.0.clone());
                        usage.insert((qid.module_id.qualified(qid.id), user_id));
                    }
                }
            }
        }

        // Collect from structs
        for struct_env in module.get_structs() {
            // From field types, including fields from variants
            for field in struct_env.get_fields() {
                let user_id =
                    UserId::Struct(struct_env.get_qualified_id(), field.get_loc().clone());
                collect_usage_from_type(&field.get_type(), &user_id, &mut usage);
            }
        }
    }

    // Apply all collected usage
    for (struct_id, user_id) in usage {
        env.add_struct_user(struct_id, user_id);
    }
}

/// Collect struct usage from a type.
fn collect_usage_from_type(
    ty: &Type,
    user_id: &UserId,
    usage: &mut BTreeSet<(QualifiedId<StructId>, UserId)>,
) {
    ty.visit(&mut |t| {
        if let Type::Struct(mid, sid, _) = t {
            usage.insert((mid.qualified(*sid), user_id.clone()));
        }
    });
}
