// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use legacy_move_compiler::shared::known_attributes::KnownAttribute;
use move_command_line_common::testing::get_compiler_exp_extension;
use move_compiler_v2::{self, run_move_compiler_for_analysis, Options};
use move_model::metadata::LanguageVersion;
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
    let options = Options {
        sources_deps: extract_test_directives(path, "// dep:")?,
        sources: vec![path.to_string_lossy().to_string()],
        dependencies: vec![],
        named_address_mapping: move_stdlib::move_stdlib_named_addresses_strings(),
        language_version: Some(LanguageVersion::latest()),
        compile_verify_code: true,
        compile_test_code: false,
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        ..Options::default()
    };
    let mut error_writer = Buffer::no_color();
    let env = run_move_compiler_for_analysis(&mut error_writer, options)?;
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
        text += &print_targets_for_test(&env, "initial translation from Move", &targets, false);

        // Run pipeline if any
        if let Some(pipeline) = pipeline_opt {
            pipeline.run(&env, &mut targets);
            let processor = pipeline.last_processor();
            if !processor.is_single_run() {
                text += &print_targets_for_test(
                    &env,
                    &format!("after pipeline `{}`", dir_name),
                    &targets,
                    false,
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
    let baseline_path = path.with_extension(get_compiler_exp_extension());
    verify_or_update_baseline(baseline_path.as_path(), &out)?;
    Ok(())
}
