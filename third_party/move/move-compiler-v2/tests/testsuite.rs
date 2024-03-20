// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use log::debug;
use move_compiler_v2::{
    annotate_units, ast_simplifier, check_and_rewrite_pipeline, cyclic_instantiation_checker,
    disassemble_compiled_units,
    env_pipeline::{
        lambda_lifter, lambda_lifter::LambdaLiftingOptions, rewrite_target::RewritingScope,
        spec_rewriter, EnvProcessorPipeline,
    },
    logging, pipeline,
    pipeline::{
        ability_processor::AbilityProcessor, avail_copies_analysis::AvailCopiesAnalysisProcessor,
        copy_propagation::CopyPropagation, dead_store_elimination::DeadStoreElimination,
        exit_state_analysis::ExitStateAnalysisProcessor,
        livevar_analysis_processor::LiveVarAnalysisProcessor,
        reference_safety_processor::ReferenceSafetyProcessor,
        uninitialized_use_checker::UninitializedUseChecker,
        unreachable_code_analysis::UnreachableCodeProcessor,
        unreachable_code_remover::UnreachableCodeRemover, variable_coalescing::VariableCoalescing,
    },
    run_bytecode_verifier, run_file_format_gen, Options,
};
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
    /// Whether compilation should stop before generating stackless bytecode,
    /// also skipping the bytecode pipeline and file format generation.
    stop_before_generating_bytecode: bool,
    /// Whether we should dump the AST after successful type check.
    dump_ast: AstDumpLevel,
    /// A sequence of transformations to run on the model.
    env_pipeline: EnvProcessorPipeline<'static>,
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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum AstDumpLevel {
    #[default]
    None,
    EndStage,
    AllStages,
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
        // The transformation pipeline on the GlobalEnv
        let mut env_pipeline =
            check_and_rewrite_pipeline(options, false, RewritingScope::CompilationTarget);
        // Add the specification rewriter for testing here as well, even though it is not run
        // as part of regular compilation, but only as part of a prover run.
        env_pipeline.add("specification rewriter", spec_rewriter::run_spec_rewriter);

        env_pipeline.add("recursive instantiation check", |env| {
            cyclic_instantiation_checker::check_cyclic_instantiations(env)
        });
        // The bytecode transformation pipeline
        let mut pipeline = FunctionTargetPipeline::default();

        // Get path to allow path-specific test configuration
        let path = path.to_string_lossy();

        // turn on simplifier unless doing no-simplifier tests.
        if path.contains("/simplifier-elimination/") {
            env_pipeline.add("simplifier", |env: &mut GlobalEnv| {
                ast_simplifier::run_simplifier(
                    env, true, // Code elimination
                )
            });
        } else if !path.contains("/no-simplifier/") {
            env_pipeline.add("simplifier", |env: &mut GlobalEnv| {
                ast_simplifier::run_simplifier(
                    env, false, // No code elimination
                )
            });
        }

        if path.contains("/inlining/")
            || path.contains("/folding/")
            || path.contains("/simplifier/")
            || path.contains("/simplifier-elimination/")
            || path.contains("/no-simplifier/")
        {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            pipeline.add_processor(Box::new(ExitStateAnalysisProcessor {}));
            pipeline.add_processor(Box::new(AbilityProcessor {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::EndStage,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/unit_test/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            options.testing = true;
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::EndStage,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/checking/") || path.contains("/parser/") {
            Self {
                stop_before_generating_bytecode: true,
                dump_ast: AstDumpLevel::EndStage,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/lambda-lifting/") {
            // Clear the transformation pipeline, only run lambda lifting
            env_pipeline = EnvProcessorPipeline::default();
            env_pipeline.add("lambda-lifting", |env: &mut GlobalEnv| {
                lambda_lifter::lift_lambdas(
                    LambdaLiftingOptions {
                        include_inline_functions: true,
                    },
                    env,
                )
            });
            Self {
                stop_before_generating_bytecode: true,
                dump_ast: AstDumpLevel::AllStages,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/bytecode-generator/") {
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::EndStage,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/file-format-generator/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            pipeline.add_processor(Box::new(ExitStateAnalysisProcessor {}));
            pipeline.add_processor(Box::new(AbilityProcessor {}));
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: true,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/cyclic-instantiation-checker") {
            Self {
                stop_before_generating_bytecode: true,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/visibility-checker/") {
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/live-var/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/reference-safety/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/abort-analysis/") {
            pipeline.add_processor(Box::new(ExitStateAnalysisProcessor {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/ability-check/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            pipeline.add_processor(Box::new(ExitStateAnalysisProcessor {}));
            pipeline.add_processor(Box::new(AbilityProcessor {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/ability-transform/") {
            // Difference to above is that we dump targets
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            pipeline.add_processor(Box::new(ExitStateAnalysisProcessor {}));
            pipeline.add_processor(Box::new(AbilityProcessor {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/copy-propagation/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            pipeline.add_processor(Box::new(ReferenceSafetyProcessor {}));
            pipeline.add_processor(Box::new(ExitStateAnalysisProcessor {}));
            pipeline.add_processor(Box::new(AbilityProcessor {}));
            pipeline.add_processor(Box::new(AvailCopiesAnalysisProcessor {})); // 4
            pipeline.add_processor(Box::new(CopyPropagation {})); // 5
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            pipeline.add_processor(Box::new(DeadStoreElimination {})); // 7
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                // Only dump with annotations after these pipeline stages.
                dump_for_only_some_stages: Some(vec![4, 5, 7]),
            }
        } else if path.contains("/uninit-use-checker/") {
            pipeline.add_processor(Box::new(UninitializedUseChecker {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/unreachable-code-remover/") {
            pipeline.add_processor(Box::new(UnreachableCodeProcessor {}));
            pipeline.add_processor(Box::new(UnreachableCodeRemover {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: false,
                dump_annotated_targets: true,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/bytecode-verify-failure/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            // Note that we do not run ability checker here, as we want to induce
            // a bytecode verification failure. The test in /bytecode-verify-failure/
            // has erroneous ability annotations.
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
                pipeline,
                generate_file_format: true,
                dump_annotated_targets: false,
                dump_for_only_some_stages: None,
            }
        } else if path.contains("/variable-coalescing/") {
            pipeline.add_processor(Box::new(LiveVarAnalysisProcessor {}));
            pipeline.add_processor(Box::new(VariableCoalescing {}));
            Self {
                stop_before_generating_bytecode: false,
                dump_ast: AstDumpLevel::None,
                env_pipeline,
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
            // Run env processor pipeline.
            if self.dump_ast == AstDumpLevel::AllStages {
                let mut out = Buffer::no_color();
                self.env_pipeline.run_and_record(&mut env, &mut out)?;
                test_output
                    .borrow_mut()
                    .push_str(&String::from_utf8_lossy(&out.into_inner()));
                ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
            } else {
                self.env_pipeline.run(&mut env);
                ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
                if ok && self.dump_ast == AstDumpLevel::EndStage {
                    test_output.borrow_mut().push_str(&format!(
                        "// -- Model dump before bytecode pipeline\n{}\n",
                        env.dump_env()
                    ));
                }
            }
        }

        if ok && !self.stop_before_generating_bytecode {
            // Run stackless bytecode generator
            let mut targets = move_compiler_v2::run_bytecode_gen(&env);
            ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
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
                ok = Self::check_diags(&mut test_output.borrow_mut(), &env);
                if ok && self.generate_file_format {
                    let units = run_file_format_gen(&env, &targets);
                    let out = &mut test_output.borrow_mut();
                    out.push_str("\n============ disassembled file-format ==================\n");
                    ok = Self::check_diags(out, &env);
                    out.push_str(&disassemble_compiled_units(&units)?);
                    if ok {
                        let annotated_units = annotate_units(units);
                        run_bytecode_verifier(&annotated_units, &mut env);
                        Self::check_diags(out, &env);
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
}

datatest_stable::harness!(test_runner, "tests", r".*\.move$");
