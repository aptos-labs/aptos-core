// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_binary_format::{binary_views::BinaryIndexedView, file_format as FF};
use move_command_line_common::files::FileHash;
use move_compiler::compiled_unit::CompiledUnit;
use move_compiler_v2::{
    pipeline::livevar_analysis_processor::LiveVarAnalysisProcessor, run_file_format_gen, Options,
};
use move_disassembler::disassembler::Disassembler;
use move_ir_types::location;
use move_model::model::GlobalEnv;
use move_prover_test_utils::{baseline_test, extract_test_directives};
use move_stackless_bytecode::{
    function_target::FunctionTarget, function_target_pipeline::FunctionTargetPipeline,
};
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
    /// Whether we should dump the AST after successful type check.
    dump_ast: bool,
    /// A sequence of bytecode processors to run for this test.
    pipeline: FunctionTargetPipeline,
    /// Whether we should generate file format from resulting bytecode
    generate_file_format: bool,
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
        let mut pipeline = FunctionTargetPipeline::default();
        if path.contains("/checking/") {
            Self {
                check_only: true,
                dump_ast: true,
                pipeline,
                generate_file_format: false,
            }
        } else if path.contains("/bytecode-generator/") {
            Self {
                check_only: false,
                dump_ast: true,
                pipeline,
                generate_file_format: false,
            }
        } else if path.contains("/file-format-generator/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            Self {
                check_only: false,
                dump_ast: false,
                pipeline,
                generate_file_format: true,
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
        let test_output = RefCell::new(String::new());

        // Run context checker
        let env = move_compiler_v2::run_checker(options)?;
        let ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
        if ok && self.dump_ast {
            let out = &mut test_output.borrow_mut();
            out.push_str("// ---- Model Dump\n");
            out.push_str(&env.dump_env());
            out.push('\n');
        }
        if ok && !self.check_only {
            // Run stackless bytecode generator
            let mut targets = move_compiler_v2::run_bytecode_gen(&env);
            let ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
            if ok {
                // Run the target pipeline.
                self.pipeline.run_with_hook(
                    &env,
                    &mut targets,
                    // Hook which is run before steps in the pipeline. Prints out initial
                    // bytecode from the generator.
                    |targets_before| {
                        let out = &mut test_output.borrow_mut();
                        Self::check_diags(out, &env);
                        out.push_str(
                            &move_stackless_bytecode::print_targets_with_annotations_for_test(
                                &env,
                                "initial bytecode",
                                targets_before,
                                Self::register_formatters,
                            ),
                        );
                    },
                    // Hook which is run after every step in the pipeline. Prints out
                    // bytecode after the processor.
                    |_, processor, targets_after| {
                        let out = &mut test_output.borrow_mut();
                        Self::check_diags(out, &env);
                        out.push_str(
                            &move_stackless_bytecode::print_targets_with_annotations_for_test(
                                &env,
                                &format!("after {}:", processor.name()),
                                targets_after,
                                Self::register_formatters,
                            ),
                        );
                    },
                );
                let ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
                if ok && self.generate_file_format {
                    let units = run_file_format_gen(&env, &targets);
                    let out = &mut test_output.borrow_mut();
                    out.push_str("\n============ disassembled file-format ==================\n");
                    Self::check_diags(out, &env);
                    for compiled_unit in units {
                        if let CompiledUnit::Module(compiled_mod) = compiled_unit {
                            let cont = Self::disassemble(&compiled_mod.module)?;
                            out.push_str(&cont)
                        }
                    }
                }
            }
        }

        // Generate/check baseline.
        let baseline_path = path.with_extension(exp_file_ext);
        baseline_test::verify_or_update_baseline(baseline_path.as_path(), &test_output.borrow())?;

        Ok(())
    }

    /// Callback from the framework to register formatters for annotations.
    fn register_formatters(target: &FunctionTarget) {
        LiveVarAnalysisProcessor::register_formatters(target)
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

    fn disassemble(module: &FF::CompiledModule) -> anyhow::Result<String> {
        let diss = Disassembler::from_view(
            BinaryIndexedView::Module(module),
            location::Loc::new(FileHash::empty(), 0, 0),
        )?;
        diss.disassemble()
    }
}

datatest_stable::harness!(test_runner, "tests", r".*\.move$");
