// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{function_target::FunctionTarget, function_target_pipeline::FunctionTargetsHolder};
use move_model::model::GlobalEnv;
use std::fmt::Write;

pub mod annotations;
pub mod borrow_analysis;
pub mod compositional_analysis;
pub mod dataflow_analysis;
pub mod dataflow_domains;
pub mod debug_instrumentation;
pub mod function_data_builder;
pub mod function_target;
pub mod function_target_pipeline;
pub mod graph;
pub mod livevar_analysis;
pub mod reaching_def_analysis;
pub mod stackless_bytecode;
pub mod stackless_bytecode_generator;
pub mod stackless_control_flow_graph;
pub mod usage_analysis;

/// An error message used for cases where a compiled module is expected to be attached
pub const COMPILED_MODULE_AVAILABLE: &str = "compiled module missing";

/// Print function targets for testing and debugging.
pub fn print_targets_for_test(
    env: &GlobalEnv,
    header: &str,
    targets: &FunctionTargetsHolder,
) -> String {
    print_targets_with_annotations_for_test(env, header, targets, |target| {
        target.register_annotation_formatters_for_test()
    })
}

/// Print function targets for testing and debugging.
pub fn print_targets_with_annotations_for_test(
    env: &GlobalEnv,
    header: &str,
    targets: &FunctionTargetsHolder,
    register_annotations: impl Fn(&FunctionTarget),
) -> String {
    let mut text = String::new();
    writeln!(&mut text, "============ {} ================", header).unwrap();
    for module_env in env.get_modules() {
        for func_env in module_env.get_functions() {
            if func_env.is_inline() {
                continue;
            }
            for (variant, target) in targets.get_targets(&func_env) {
                if !target.data.code.is_empty() || target.func_env.is_native_or_intrinsic() {
                    register_annotations(&target);
                    writeln!(&mut text, "\n[variant {}]\n{}", variant, target).unwrap();
                }
            }
        }
    }
    text
}
