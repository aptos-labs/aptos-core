// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module checks whether closure expressions are valid, which is done after type inference
//! and lambda lifting. Current checks:
//!
//! - The closure satisfies the ability requirements of it's inferred type. For the
//!   definition of closure abilities, see
//!   [AIP-112](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-112.md).
//! - The closure does not capture references, as this is currently not allowed.
//! - In a script, the closure cannot have a lambda lifted function.
//! ```

use crate::env_pipeline::lambda_lifter;
use move_binary_format::file_format::Visibility;
use move_core_types::ability::Ability;
use move_model::{
    ast::{ExpData, Operation},
    model::GlobalEnv,
    ty::Type,
    well_known,
};

/// Checks various properties of lambda expressions in all target module functions.
pub fn check_closures(env: &GlobalEnv) {
    for module_env in env.get_primary_target_modules() {
        let is_script_module = module_env.is_script_module();
        for fun_env in module_env.get_functions() {
            if let Some(def) = fun_env.get_def() {
                def.visit_pre_order(&mut |e| {
                    if let ExpData::Call(id, Operation::Closure(mid, fid, _), args) = e {
                        let mut context_ty = env.get_node_type(*id);
                        let mut function_wrapper_ty = None;
                        if let Some(ty) = context_ty.get_function_wrapper_ty(env) {
                            function_wrapper_ty = Some(context_ty);
                            context_ty = ty;
                        }
                        let required_abilities =
                            env.type_abilities(&context_ty, fun_env.get_type_parameters_ref());
                        let fun_env = env.get_function(mid.qualified(*fid));
                        let is_lambda_lifted = lambda_lifter::is_lambda_lifted_fun(&fun_env);
                        // The function itself has all abilities except `store`, which it only
                        // has if it is public. Notice that since required_abilities is derived
                        // from the function type of the closure, it cannot have `key` ability.
                        if required_abilities.has_ability(Ability::Store)
                            && fun_env.visibility() != Visibility::Public
                            && !fun_env.has_attribute(|attr| {
                                env.symbol_pool().string(attr.name()).as_str()
                                    == well_known::PERSISTENT_ATTRIBUTE
                            })
                        {
                            env.error_with_notes(
                                &env.get_node_loc(*id),
                                &format!(
                                    "function {} is missing the `store` ability",
                                    if is_lambda_lifted {
                                        "resulting from lambda lifting".to_string()
                                    } else {
                                        format!("`{}`", fun_env.get_full_name_str())
                                    },
                                ),
                                vec![
                                    if is_lambda_lifted {
                                        "lambda cannot be reduced to partial application of \
                                        existing function"
                                            .to_string()
                                    } else {
                                        "only public functions or functions with the \
                                        `#[persistent]` attribute can be stored"
                                            .to_string()
                                    },
                                    format!(
                                        "expected function type: `{}`",
                                        context_ty.display(&fun_env.get_type_display_ctx())
                                    ),
                                ],
                            );
                        }

                        // All captured arguments must (a) have least the required abilities
                        // (b) must not be references
                        let wrapper_msg = || {
                            if let Some(ty) = &function_wrapper_ty {
                                format!(
                                    " (wrapped type of `{}`)",
                                    ty.display(&fun_env.get_type_display_ctx())
                                )
                            } else {
                                "".to_owned()
                            }
                        };
                        for captured in args {
                            let captured_ty = env.get_node_type(captured.node_id());
                            // when capturing a value that contains option, we need to generate a warning
                            // After refactoring option type to use enum, we can lift this limitation
                            // TODO: remove it after option type is refactored to use enum
                            if contains_option_type(env, &captured_ty) {
                                env.warning(&env.get_node_loc(captured.node_id()), "capturing option values is currently not supported");
                            }
                            if captured_ty.is_reference() {
                                env.error(
                                    &env.get_node_loc(captured.node_id()),
                                    &format!(
                                        "captured value cannot be a reference, but has type `{}`{}",
                                        captured_ty.display(&fun_env.get_type_display_ctx()),
                                        wrapper_msg()
                                    ),
                                )
                            }
                            let arg_ty_abilities = env.type_abilities(
                                &env.get_node_type(captured.node_id()),
                                fun_env.get_type_parameters_ref(),
                            );
                            let missing = required_abilities.setminus(arg_ty_abilities);
                            if !missing.is_empty() {
                                env.error_with_notes(
                                    &env.get_node_loc(captured.node_id()),
                                    &format!("captured value is missing abilities `{}`", missing,),
                                    vec![format!(
                                        "expected function type: `{}`{}",
                                        context_ty.display(&fun_env.get_type_display_ctx()),
                                        wrapper_msg()
                                    )],
                                )
                            }
                        }

                        // (d) Scripts cannot have closures with lambda lifted functions.
                        if is_script_module && is_lambda_lifted {
                            env.error_with_notes(
                                &env.get_node_loc(*id),
                                "lambda lifting is not allowed in scripts",
                                vec!["lambda cannot be reduced to partial application of an existing function".to_string()],
                            );
                        }
                    }

                    // Continue visiting
                    true
                });
            }
        }
    }
}

/// Check if the type contains an option type
/// Note that this function does not find contained options within reference types/function types
/// because it is used for checking captured values only
fn contains_option_type(env: &GlobalEnv, ty: &Type) -> bool {
    match ty {
        Type::Vector(ty) => contains_option_type(env, ty),
        Type::Struct(mid, sid, tys) => {
            let struct_env = env.get_module(*mid).into_struct(*sid);
            if struct_env.is_option_type() {
                return true;
            }
            if struct_env.has_variants() {
                for variant in struct_env.get_variants() {
                    for field in struct_env.get_fields_of_variant(variant) {
                        if contains_option_type(env, &field.get_type()) {
                            return true;
                        }
                    }
                }
            } else {
                for field in struct_env.get_fields() {
                    if contains_option_type(env, &field.get_type()) {
                        return true;
                    }
                }
            }
            tys.iter()
                .zip(struct_env.get_type_parameters().iter())
                .filter(|(_, param)| !param.1.is_phantom)
                .any(|(t, _)| contains_option_type(env, t))
        },
        // since fun params and result does not appear in layout, we can safely return false,
        Type::Fun(..) => false,
        Type::Primitive(..) => false,
        // compiler error will be generated separately
        // we are looking at option values that are captured, and because references cannot be captured,
        // we do not have to recurse here.
        Type::Reference(..) => false,
        Type::TypeParameter(..) => false,
        _ => unreachable!(
            "ICE: argument with type `{}` should not appear in this context",
            ty.display(&env.get_type_display_ctx())
        ),
    }
}
