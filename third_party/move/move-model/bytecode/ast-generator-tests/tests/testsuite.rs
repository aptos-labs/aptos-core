// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::term::termcolor::Buffer;
use move_compiler_v2::{logging, run_move_compiler_for_analysis, Options};
use move_model::{ast::Exp, metadata::LanguageVersion, model::GlobalEnv, sourcifier::Sourcifier};
use move_prover_test_utils::{baseline_test, extract_test_directives};
use move_stackless_bytecode::{
    astifier,
    function_target::FunctionTarget,
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// Extension for expected output files
pub const EXP_EXT: &str = "exp";

datatest_stable::harness!(test_runner, "tests", r".*\.move$");

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    logging::setup_logging_for_testing(None);
    let path_str = path.display().to_string();
    let mut options = Options {
        sources_deps: extract_test_directives(path, "// dep:")?,
        sources: vec![path_str.clone()],
        dependencies: if extract_test_directives(path, "// no-stdlib")?.is_empty() {
            vec![path_from_crate_root("../../../move-stdlib/sources")]
        } else {
            vec![]
        },
        named_address_mapping: vec!["std=0x1".to_string()],
        ..Options::default()
    };
    options = options.set_language_version(LanguageVersion::latest_stable());
    let mut test_output = String::new();
    let mut error_writer = Buffer::no_color();
    match run_move_compiler_for_analysis(&mut error_writer, options) {
        Err(_) => {
            test_output.push_str(&format!(
                "--- Aborting with compilation errors:\n{}\n",
                String::from_utf8_lossy(&error_writer.into_inner())
            ));
        },
        Ok(mut env) => {
            let targets = create_targets(&env);
            let mut modules = BTreeSet::new();
            for fun_id in targets.get_funs() {
                let fun_env = env.get_function(fun_id);
                modules.insert(fun_env.module_env.get_id());
                let def = generate_output(
                    &targets.get_target(&env.get_function(fun_id), &FunctionVariant::Baseline),
                    &mut test_output,
                );
                if let Some(def) = def {
                    env.set_function_def(fun_id, def);
                }
            }
            let sourcifier = Sourcifier::new(&env, true);
            for mid in modules {
                sourcifier.print_module(mid)
            }
            test_output += &format!(
                "=== Sourcified Output ============================================\n{}",
                sourcifier.result()
            )
        },
    }
    // Generate/check baseline.
    let baseline_path = path.with_extension(EXP_EXT);
    baseline_test::verify_or_update_baseline(baseline_path.as_path(), &test_output)?;
    Ok(())
}

fn generate_output(target: &FunctionTarget, test_output: &mut String) -> Option<Exp> {
    *test_output += &format!(
        "\n=== Processing {} =====================================================\n",
        target.func_env.get_full_name_str()
    );
    *test_output += &format!(
        "--- Source\n{}\n",
        target
            .global_env()
            .get_source(&target.get_loc())
            .unwrap_or("UNKNOWN")
    );

    *test_output += &format!("\n--- Stackless Bytecode\n{}\n", target);

    let Some(exp) = astifier::generate_ast_raw(target) else {
        *test_output += "--- Raw Generated AST\nFAILED\n";
        return None;
    };
    *test_output += &format!(
        "--- Raw Generated AST\n{}\n\n",
        exp.display_for_fun(target.func_env)
    );
    let exp = astifier::transform_assigns(target, exp);
    *test_output += &format!(
        "--- Assign-Transformed Generated AST\n{}\n\n",
        exp.display_for_fun(target.func_env)
    );
    let exp = astifier::transform_conditionals(target, exp);
    *test_output += &format!(
        "--- If-Transformed Generated AST\n{}\n\n",
        exp.display_for_fun(target.func_env)
    );
    let exp = astifier::bind_free_vars(target, exp);
    *test_output += &format!(
        "--- Var-Bound Generated AST\n{}\n\n",
        exp.display_for_fun(target.func_env)
    );
    Some(exp)
}

/// Create function targets with stackless bytecode for modules which are target.
/// This decompiles Move binary format into stackless bytecode.
fn create_targets(env: &GlobalEnv) -> FunctionTargetsHolder {
    let mut targets = FunctionTargetsHolder::default();
    for module_env in env.get_modules() {
        if module_env.is_primary_target() {
            for func_env in module_env.get_functions() {
                targets.add_target(&func_env)
            }
        }
    }
    targets
}

/// Returns a path relative to the crate root.
fn path_from_crate_root(path: &str) -> String {
    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}
