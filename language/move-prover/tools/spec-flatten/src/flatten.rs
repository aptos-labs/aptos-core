// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use bytecode::{function_target::FunctionTarget, function_target_pipeline::FunctionTargetsHolder};

pub(crate) fn flatten_spec<'env>(
    fun_target: FunctionTarget<'env>,
    _targets: &'env FunctionTargetsHolder,
) {
    let env = fun_target.global_env();
    let fun_env = fun_target.func_env;
    let spec = fun_target.get_spec();

    if !spec.conditions.is_empty() {
        println!("fun {}\n{}", fun_env.get_full_name_str(), env.display(spec));
    }
}
