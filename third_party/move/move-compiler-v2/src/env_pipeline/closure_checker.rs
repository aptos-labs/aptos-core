// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module checks whether closure expressions are valid, which is done after type inference
//! and lambda lifting. Current checks:
//!
//! - The closure satisfies the ability requirements of it's inferred type. For the
//!   definition of closure abilities, see
//!   [AIP-112](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-112.md).
//! - The closure does not capture references, as this is currently not allowed.
//! ```

use crate::env_pipeline::lambda_lifter;
use move_binary_format::file_format::Visibility;
use move_core_types::ability::Ability;
use move_model::{
    ast::{ExpData, Operation},
    model::GlobalEnv,
    well_known,
};

/// Checks lambda expression abilities in all target module functions.
pub fn check_closures(env: &GlobalEnv) {
    for module_env in env.get_primary_target_modules() {
        for fun_env in module_env.get_functions() {
            if let Some(def) = fun_env.get_def() {
                def.visit_pre_order(&mut |e| {
                    if let ExpData::Call(id, Operation::Closure(mid, fid, _), args) = e {
                        let context_ty = env.get_node_type(*id);
                        let required_abilities =
                            env.type_abilities(&context_ty, fun_env.get_type_parameters_ref());

                        let fun_env = env.get_function(mid.qualified(*fid));
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
                            let is_lambda_lifted = lambda_lifter::is_lambda_lifted_fun(&fun_env);
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
                        for captured in args {
                            let captured_ty = env.get_node_type(captured.node_id());
                            if captured_ty.is_reference() {
                                env.error(
                                    &env.get_node_loc(captured.node_id()),
                                    &format!(
                                        "captured value cannot be a reference, but has type `{}`",
                                        captured_ty.display(&fun_env.get_type_display_ctx())
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
                                    &format!("captured value is missing abilities `{}`", missing),
                                    vec![format!(
                                        "expected function type: `{}`",
                                        context_ty.display(&fun_env.get_type_display_ctx())
                                    )],
                                )
                            }
                        }
                    }

                    // Continue visiting
                    true
                });
            }
        }
    }
}
