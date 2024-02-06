// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use log::{debug, trace};
use move_binary_format::binary_views::BinaryIndexedView;
use move_command_line_common::files::FileHash;
use move_compiler::compiled_unit::CompiledUnit;
use move_compiler_v2::{
    flow_insensitive_checkers, function_checker, inliner, logging, pipeline,
    pipeline::{
        ability_checker::AbilityChecker, avail_copies_analysis::AvailCopiesAnalysisProcessor,
        copy_propagation::CopyPropagation, dead_store_elimination::DeadStoreElimination,
        explicit_drop::ExplicitDrop, livevar_analysis_processor::LiveVarAnalysisProcessor,
        reference_safety_processor::ReferenceSafetyProcessor,
        uninitialized_use_checker::UninitializedUseChecker,
        unreachable_code_analysis::UnreachableCodeProcessor,
        unreachable_code_remover::UnreachableCodeRemover, visibility_checker::VisibilityChecker,
    },
    run_file_format_gen, Options,
};
use move_disassembler::disassembler::Disassembler;
use move_ir_types::location;
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
    type_check_only: bool,
    /// Whether we should dump the AST after successful type check.
    dump_ast: bool,
    /// A sequence of bytecode processors to run for this test.
    pipeline: FunctionTargetPipeline,
    /// Whether we should generate file format from resulting bytecode.
    generate_file_format: bool,
    /// Whether we should dump annotated targets for each stage of the pipeline.
    dump_annotated_targets: bool,
    /// Optionally, dump annotated targets for only certain stages of the pipeline.
    /// If None, dump annotated targets for all stages.
    /// If Some(list), dump annotated targets for pipeline stages whose index is in the list.
    /// If `dump_annotated_targets` is false, this field is ignored.
    /// Note: the pipeline stages are numbered starting from 0.
    dump_for_only_some_stages: Option<Vec<usize>>,
}

fn path_from_crate_root(path: &str) -> String {
    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    logging::setup_logging_for_testing();
    let mut experiments = extract_test_directives(path, "// experiment:")?;
    if experiments.is_empty() {
        // If there is no experiment, use "" as the 'default' experiment.
        experiments.push("".to_string()) // default experiment
    }
    let mut sources = extract_test_directives(path, "// dep:")?;
    sources.push(path.to_string_lossy().to_string());
    let deps = vec![path_from_crate_root("../move-stdlib/sources")];
    let path_string = path.to_string_lossy();
    let warn_unused = path_string.contains("unused");

    // For each experiment, run the test at `path`.
    for experiment in experiments {
        let mut options = Options {
            testing: true,
            sources: sources.clone(),
            dependencies: deps.clone(),
            named_address_mapping: vec!["std=0x1".to_string()],
            warn_unused,
            ..Options::default()
        };
        TestConfig::get_config_from_path(path, &mut options).run(path, experiment, options)?
    }
    Ok(())
}

impl TestConfig {
    fn get_config_from_path(path: &Path, options: &mut Options) -> TestConfig {
        // Construct options, compiler and collect output.
        let path = path.to_string_lossy();
        let verbose = cfg!(feature = "verbose-debug-print");
        let mut pipeline = FunctionTargetPipeline::default();
        if path.contains("/inlining/bug_11112") || path.contains("/inlining/bug_9717_looponly") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            pipeline.add_processor(Box::new(VisibilityChecker {}));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            Self {
                type_check_only: false,
                dump_ast: true,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/inlining/") || path.contains("/folding/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            pipeline.add_processor(Box::new(VisibilityChecker {}));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            Self {
                type_check_only: false,
                dump_ast: true,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: verbose,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/inlining/") {
            pipeline.add_processor(Box::new(VisibilityChecker {}));
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            pipeline.add_processor(Box::new(ExplicitDrop {}));
            pipeline.add_processor(Box::new(AbilityChecker {}));
            Self {
                type_check_only: false,
                dump_ast: true,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: verbose,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/unit_test/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            pipeline.add_processor(Box::new(VisibilityChecker {}));
            options.testing = true;
            Self {
                type_check_only: false,
                dump_ast: true,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: verbose,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/checking/") {
            Self {
                type_check_only: true,
                dump_ast: true,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: verbose,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/bytecode-generator/") {
            Self {
                type_check_only: false,
                dump_ast: true,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/file-format-generator/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            Self {
                type_check_only: false,
                dump_ast: false,
                pipeline,
                generate_file_format: true,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/visibility-checker/") {
            pipeline.add_processor(Box::new(VisibilityChecker {}));
            Self {
                type_check_only: false,
                dump_ast: false,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: verbose,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/live-var/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            Self {
                type_check_only: false,
                dump_ast: false,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/reference-safety/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            Self {
                type_check_only: false,
                dump_ast: verbose,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: verbose,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/explicit-drop/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            pipeline.add_processor(Box::new(ExplicitDrop {}));
            Self {
                type_check_only: false,
                dump_ast: verbose,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/ability-checker/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: true,
            }));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            pipeline.add_processor(Box::new(ExplicitDrop {}));
            pipeline.add_processor(Box::new(AbilityChecker {}));
            Self {
                type_check_only: false,
                dump_ast: false,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/copy-propagation/") {
            pipeline.add_processor(Box::new(AvailCopiesAnalysisProcessor {})); // 0
            pipeline.add_processor(Box::new(CopyPropagation {})); // 1
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: false,
            }));
            pipeline.add_processor(Box::new(DeadStoreElimination {})); // 3
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {
                with_copy_inference: false,
            }));
            Self {
                type_check_only: false,
                dump_ast: false,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                // Only dump with annotations after these pipeline stages.
                dump_for_only_some_stages: Some(vec![0, 1, 3]),
            }
        } else if path.contains("/uninit-use-checker/") {
            pipeline.add_processor(Box::new(UninitializedUseChecker {}));
            Self {
                type_check_only: false,
                dump_ast: false,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/unreachable-code-remover/") {
            pipeline.add_processor(Box::new(UnreachableCodeProcessor {}));
            pipeline.add_processor(Box::new(UnreachableCodeRemover {}));
            Self {
                type_check_only: false,
                dump_ast: false,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
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
        let mut env = move_compiler_v2::run_checker(options.clone())?;
        let mut ok = Self::check_diags(&mut test_output.borrow_mut(), &env);

        if ok {
            trace!("After error check, GlobalEnv={}", env.dump_env());
            // Flow-insensitive checks on AST
            flow_insensitive_checkers::check_for_unused_vars_and_params(&mut env);
            function_checker::check_for_function_typed_parameters(&mut env);
            function_checker::check_access_and_use(&mut env);
            ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
        }
        if ok {
            trace!(
                "After flow-insensitive checks, GlobalEnv={}",
                env.dump_env()
            );
            // Run inlining.
            inliner::run_inlining(&mut env);
            ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
        }
        if ok {
            trace!("After inlining, GlobalEnv={}", env.dump_env());
        }

        if ok && self.dump_ast {
            let out = &mut test_output.borrow_mut();
            out.push_str("// ---- Model Dump\n");
            out.push_str(&env.dump_env());
            out.push('\n');
        }
        if ok && !self.type_check_only {
            // Run stackless bytecode generator
            let mut targets = move_compiler_v2::run_bytecode_gen(&env);
            let ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
            if ok {
                // Run the target pipeline.
                self.pipeline.run_with_hook(
                    &env,
                    &mut targets,
                    // Hook which is run before steps in the pipeline. Prints out initial
                    // bytecode from the generator, if requested.
                    |targets_before| {
                        let out = &mut test_output.borrow_mut();
                        Self::check_diags(out, &env);
                        if self.dump_annotated_targets {
                            out.push_str(
                                &move_stackless_bytecode::print_targets_with_annotations_for_test(
                                    &env,
                                    "initial bytecode",
                                    targets_before,
                                    &pipeline::register_formatters,
                                    false,
                                ),
                            );
                        }
                        debug!(
                            "{}",
                            &move_stackless_bytecode::print_targets_with_annotations_for_test(
                                &env,
                                "initial bytecode",
                                targets_before,
                                &pipeline::register_formatters,
                                true,
                            ),
                        )
                    },
                    // Hook which is run after every step in the pipeline. Prints out
                    // bytecode after the processor, if requested.
                    |i, processor, targets_after| {
                        let out = &mut test_output.borrow_mut();
                        Self::check_diags(out, &env);
                        // Note that `i` starts at 1.
                        let title = format!("after {}:", processor.name());
                        let stage_dump_enabled = self.dump_for_only_some_stages.is_none()
                            || self
                                .dump_for_only_some_stages
                                .as_ref()
                                .is_some_and(|list| list.contains(&(i - 1)));
                        if self.dump_annotated_targets && stage_dump_enabled {
                            out.push_str(
                                &move_stackless_bytecode::print_targets_with_annotations_for_test(
                                    &env,
                                    &title,
                                    targets_after,
                                    &pipeline::register_formatters,
                                    false,
                                ),
                            );
                        }
                        if stage_dump_enabled {
                            debug!(
                                "{}",
                                &move_stackless_bytecode::print_targets_with_annotations_for_test(
                                    &env,
                                    &title,
                                    targets_after,
                                    &pipeline::register_formatters,
                                    true,
                                )
                            )
                        }
                    },
                );
                let ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
                if ok && self.generate_file_format {
                    let units = run_file_format_gen(&env, &targets);
                    let out = &mut test_output.borrow_mut();
                    out.push_str("\n============ disassembled file-format ==================\n");
                    Self::check_diags(out, &env);
                    for compiled_unit in units {
                        let disassembled = match compiled_unit {
                            CompiledUnit::Module(module) => {
                                Self::disassemble(BinaryIndexedView::Module(&module.module))?
                            },
                            CompiledUnit::Script(script) => {
                                Self::disassemble(BinaryIndexedView::Script(&script.script))?
                            },
                        };
                        out.push_str(&disassembled);
                    }
                }
            }
        }

        // Generate/check baseline.
        let baseline_path = path.with_extension(exp_file_ext);
        baseline_test::verify_or_update_baseline(baseline_path.as_path(), &test_output.borrow())?;

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

    fn disassemble(view: BinaryIndexedView) -> anyhow::Result<String> {
        let diss = Disassembler::from_view(view, location::Loc::new(FileHash::empty(), 0, 0))?;
        diss.disassemble()
    }
}

datatest_stable::harness!(test_runner, "tests", r".*\.move$");
