// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_compiler_v2::Options;
use move_model::model::GlobalEnv;
use move_prover_test_utils::{baseline_test, extract_test_directives};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetPipeline;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

/// Extension for expected output files
pub const EXP_EXT: &str = "exp";

/// Configuration for a set of tests.
#[derive(Default)]
struct TestConfig {
    /// Whether only type check should be run.
    check_only: bool,
    /// A sequence of bytecode processors to run for this test.
    pipeline: FunctionTargetPipeline,
}

fn path_from_crate_root(path: &str) -> String {
    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let mut experiments = extract_test_directives(path, "// experiment:")?;
    if experiments.is_empty() {
        // If there is no experiment, use "" as the 'default' experiment.
        experiments.push("".to_string()) // default experiment
    }
    let mut sources = extract_test_directives(path, "// dep:")?;
    sources.push(path.to_string_lossy().to_string());
    let deps = vec![path_from_crate_root("../move-stdlib/sources")];

    // For each experiment, run the test at `path`.
    for experiment in experiments {
        // Construct options, compiler and collect output.
        let options = Options {
            testing: true,
            sources: sources.clone(),
            dependencies: deps.clone(),
            named_address_mapping: vec!["std=0x1".to_string()],
            ..Options::default()
        };
        TestConfig::get_config_from_path(path).run(path, experiment, options)?
    }
    Ok(())
}

impl TestConfig {
    fn get_config_from_path(path: &Path) -> TestConfig {
        let path = path.to_string_lossy();
        if path.contains("/checking/") {
            Self {
                check_only: true,
                pipeline: FunctionTargetPipeline::default(),
            }
        } else if path.contains("/bytecode-generator/") {
            Self {
                check_only: false,
                pipeline: FunctionTargetPipeline::default(),
            }
        } else {
            panic!(
                "unexpected test path `{}`, cannot derive configuration",
                path
            )
        }
    }

    fn run(self, path: &Path, experiment: String, mut options: Options) -> anyhow::Result<()> {
        let exp_file_ext = if experiment.is_empty() {
            EXP_EXT.to_string()
        } else {
            let ext = format!("{}.{}", EXP_EXT, experiment);
            options.experiments.push(experiment);
            ext
        };

        // Putting the generated test baseline into a Refcell to avoid problems with mut borrow
        // in closures.
        let baseline = RefCell::new(String::new());

        // Run context checker
        let env = move_compiler_v2::run_checker(options)?;
        let ok = Self::check_diags(&mut baseline.borrow_mut(), &env);
        if ok && !self.check_only {
            // Run stackless bytecode generator
            let mut targets = move_compiler_v2::run_bytecode_gen(&env);
            // Run the target pipeline.
            self.pipeline.run_with_hook(
                &env,
                &mut targets,
                // Hook which is run before steps in the pipeline. Prints out initial
                // bytecode from the generator.
                |targets_before| {
                    let baseline = &mut baseline.borrow_mut();
                    Self::check_diags(baseline, &env);
                    baseline.push_str(&move_stackless_bytecode::print_targets_for_test(
                        &env,
                        "initial bytecode",
                        targets_before,
                    ));
                },
                // Hook which is run after every step in the pipeline. Prints out
                // bytecode after the processor.
                |_, processor, targets_after| {
                    let baseline = &mut baseline.borrow_mut();
                    Self::check_diags(baseline, &env);
                    baseline.push_str(&move_stackless_bytecode::print_targets_for_test(
                        &env,
                        &format!("after {}:", processor.name()),
                        targets_after,
                    ));
                },
            );
        }

        // Generate/check baseline.
        let baseline_path = path.with_extension(exp_file_ext);
        baseline_test::verify_or_update_baseline(baseline_path.as_path(), &baseline.borrow())?;

        Ok(())
    }

    fn check_diags(baseline: &mut String, env: &GlobalEnv) -> bool {
        let mut error_writer = Buffer::no_color();
        env.report_diag(&mut error_writer, Severity::Note);
        let diag = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
        if !diag.is_empty() {
            *baseline += &format!("\nDiagnostics:\n{}", diag);
        }
        let ok = !env.has_errors();
        env.clear_diag();
        ok
    }
}

datatest_stable::harness!(test_runner, "tests", r".*\.move$");
