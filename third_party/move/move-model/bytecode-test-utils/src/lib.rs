// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_command_line_common::testing::EXP_EXT;
use move_compiler::shared::{known_attributes::KnownAttribute, PackagePaths};
use move_model::{model::GlobalEnv, options::ModelBuilderOptions, run_model_builder_with_options};
use move_prover_test_utils::{baseline_test::verify_or_update_baseline, extract_test_directives};
use move_stackless_bytecode::{
    function_target_pipeline::{
        FunctionTargetPipeline, FunctionTargetsHolder, ProcessorResultDisplay,
    },
    print_targets_for_test,
};
use std::path::Path;

/// A test runner which dumps annotated bytecode and can be used for implementing a `datatest`
/// runner. In addition to the path where the Move source resides, an optional processing
/// pipeline is passed to establish the state to be tested. This will dump the initial
/// bytecode and the result of the pipeline in a baseline file.
/// The Move source file can use comments of the form `// dep: file.move` to add additional
/// sources.
pub fn test_runner(
    path: &Path,
    pipeline_opt: Option<FunctionTargetPipeline>,
) -> anyhow::Result<()> {
    let mut sources = extract_test_directives(path, "// dep:")?;
    sources.push(path.to_string_lossy().to_string());
    let env: GlobalEnv = run_model_builder_with_options(
        vec![PackagePaths {
            name: None,
            paths: sources,
            named_address_map: move_stdlib::move_stdlib_named_addresses(),
        }],
        vec![],
        ModelBuilderOptions::default(),
        false,
        KnownAttribute::get_all_attribute_names(),
    )?;
    let out = if env.has_errors() {
        let mut error_writer = Buffer::no_color();
        env.report_diag(&mut error_writer, Severity::Error);
        String::from_utf8_lossy(&error_writer.into_inner()).to_string()
    } else {
        let dir_name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|p| p.to_str())
            .ok_or_else(|| anyhow!("bad file name"))?;

        // Initialize and print function targets
        let mut text = String::new();
        let mut targets = FunctionTargetsHolder::default();
        for module_env in env.get_modules() {
            for func_env in module_env.get_functions() {
                targets.add_target(&func_env);
            }
        }
        text += &print_targets_for_test(&env, "initial translation from Move", &targets);

        // Run pipeline if any
        if let Some(pipeline) = pipeline_opt {
            pipeline.run(&env, &mut targets);
            let processor = pipeline.last_processor();
            if !processor.is_single_run() {
                text += &print_targets_for_test(
                    &env,
                    &format!("after pipeline `{}`", dir_name),
                    &targets,
                );
            }
            text += &ProcessorResultDisplay {
                env: &env,
                targets: &targets,
                processor,
            }
            .to_string();
        }
        // add Warning and Error diagnostics to output
        let mut error_writer = Buffer::no_color();
        if env.has_errors() || env.has_warnings() {
            env.report_diag(&mut error_writer, Severity::Warning);
            text += "============ Diagnostics ================\n";
            text += &String::from_utf8_lossy(&error_writer.into_inner());
        }
        text
    };
    let baseline_path = path.with_extension(EXP_EXT);
    verify_or_update_baseline(baseline_path.as_path(), &out)?;
    Ok(())
}
