// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use std::collections::BTreeMap;

use bytecode::function_target_pipeline::{FunctionVariant, VerificationFlavor};
use move_model::ast::SpecBlockTarget;

mod ast_print;
mod flatten;
mod options;
mod workflow;

pub use ast_print::SpecPrinter;
pub use options::FlattenOptions;

//**************************************************************************************************
// Entrypoint
//**************************************************************************************************

pub fn run(options: &FlattenOptions) -> Result<()> {
    let (env, targets) = workflow::prepare(options)?;

    // make sure the original verification works
    let proved = workflow::prove(options, &env, &targets)?;
    if !proved {
        return Err(anyhow!("Original proof is not successful"));
    }

    // flatten spec in target modules
    let mut flattened_specs = BTreeMap::new();
    for (fun_id, variant) in targets.get_funs_and_variants() {
        if !matches!(
            variant,
            FunctionVariant::Verification(VerificationFlavor::Regular)
        ) {
            // only care for functions that have the regular verification variant
            continue;
        }

        let fun_env = env.get_function(fun_id);
        if !fun_env.module_env.is_target() {
            // only run on specs in target module
            continue;
        }
        match &options.target {
            None => {
                if !fun_env.has_unknown_callers() {
                    // only run on specs for external-facing functions
                    continue;
                }
            }
            Some(target) => {
                if fun_env.get_simple_name_string().as_ref() != target {
                    // only run on matched function name
                    continue;
                }
            }
        }

        let target = targets.get_target(&fun_env, &variant);
        let new_spec = flatten::flatten_spec(options, target, &targets)?;
        flattened_specs.insert(fun_id, new_spec);
    }

    // dump the result
    for (fun_id, spec) in flattened_specs {
        let fun_env = env.get_function(fun_id);
        let fun_scope = SpecBlockTarget::Function(fun_id.module_id, fun_id.id);
        let printer = SpecPrinter::new(&env, &fun_scope);
        if !spec.conditions.is_empty() {
            println!("fun {}{{", fun_env.get_full_name_str());
            for cond in &spec.conditions {
                println!("\t{}", SpecPrinter::convert(printer.print_condition(cond)));
            }
            println!("}}");
        }
    }

    // everything is OK
    Ok(())
}
