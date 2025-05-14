// Copyright (c) Aptos Foundation
// Parts of the project are originally copyright (c) Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod bytecode_generator;
pub mod diagnostics;
pub mod env_pipeline;
mod experiments;
pub mod external_checks;
pub mod file_format_generator;
pub mod lint_common;
pub mod logging;
pub mod options;
pub mod pipeline;
pub mod plan_builder;

use crate::{
    diagnostics::Emitter,
    env_pipeline::{
        acquires_checker, ast_simplifier, closure_checker, cmp_rewriter,
        cyclic_instantiation_checker, flow_insensitive_checkers, function_checker, inliner,
        lambda_lifter, lambda_lifter::LambdaLiftingOptions, model_ast_lints,
        recursive_struct_checker, rewrite_target::RewritingScope, seqs_in_binop_checker,
        spec_checker, spec_rewriter, unused_params_checker, EnvProcessorPipeline,
    },
    pipeline::{
        ability_processor::AbilityProcessor,
        control_flow_graph_simplifier::ControlFlowGraphSimplifier,
        dead_store_elimination::DeadStoreElimination,
        exit_state_analysis::ExitStateAnalysisProcessor,
        flush_writes_processor::FlushWritesProcessor,
        lint_processor::LintProcessor,
        livevar_analysis_processor::LiveVarAnalysisProcessor,
        reference_safety::{reference_safety_processor_v2, reference_safety_processor_v3},
        split_critical_edges_processor::SplitCriticalEdgesProcessor,
        uninitialized_use_checker::UninitializedUseChecker,
        unreachable_code_analysis::UnreachableCodeProcessor,
        unreachable_code_remover::UnreachableCodeRemover,
        unused_assignment_checker::UnusedAssignmentChecker,
        variable_coalescing::VariableCoalescing,
    },
};
use codespan_reporting::{
    diagnostic::Severity,
    term::termcolor::{ColorChoice, StandardStream, WriteColor},
};
pub use experiments::{Experiment, EXPERIMENTS};
use legacy_move_compiler::{
    command_line,
    compiled_unit::{
        AnnotatedCompiledModule, AnnotatedCompiledScript, AnnotatedCompiledUnit, CompiledUnit,
        FunctionInfo, NamedCompiledModule, NamedCompiledScript,
    },
    diagnostics::FilesSourceText,
    shared::known_attributes::KnownAttribute,
};
use log::{debug, info, log_enabled, Level};
use move_binary_format::errors::VMError;
use move_bytecode_source_map::source_map::SourceMap;
use move_core_types::vm_status::StatusType;
use move_disassembler::disassembler::Disassembler;
use move_model::{
    metadata::LanguageVersion,
    model::{GlobalEnv, Loc, MoveIrLoc},
    PackageInfo,
};
use move_stackless_bytecode::function_target_pipeline::{
    FunctionTargetPipeline, FunctionTargetsHolder, FunctionVariant,
};
use move_symbol_pool::Symbol;
pub use options::Options;
use std::{collections::BTreeSet, path::Path};

const DEBUG: bool = false;
const COMPILER_BUG_REPORT_MSG: &str =
    "please consider reporting this issue (see https://aptos.dev/en/build/smart-contracts/compiler_v2#reporting-an-issue)";

/// Run Move compiler and print errors to stderr.
pub fn run_move_compiler_to_stderr(
    options: Options,
) -> anyhow::Result<(GlobalEnv, Vec<AnnotatedCompiledUnit>)> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let mut emitter = options.error_emitter(&mut stderr);
    run_move_compiler(emitter.as_mut(), options)
}

/// Run move compiler and print errors to given writer. Returns the set of compiled units.
pub fn run_move_compiler<E>(
    emitter: &mut E,
    options: Options,
) -> anyhow::Result<(GlobalEnv, Vec<AnnotatedCompiledUnit>)>
where
    E: Emitter + ?Sized,
{
    logging::setup_logging();
    info!("Move Compiler v2");

    // Run context check.
    let mut env = run_checker(options.clone())?;
    check_errors(&env, emitter, "context checking errors")?;

    // Run a AST pipeline of checks and (non-optimization) transforms.
    env_check_and_transform_pipeline(&options).run(&mut env);
    check_errors(&env, emitter, "env checking errors")?;

    if options.experiment_on(Experiment::STOP_BEFORE_STACKLESS_BYTECODE) {
        std::process::exit(if env.has_warnings() { 1 } else { 0 })
    }

    // Run stackless-bytecode generator.
    let mut targets = run_stackless_bytecode_gen(&env);
    check_errors(&env, emitter, "stackless bytecode generation errors")?;

    if DEBUG {
        debug!("After bytecode_gen, GlobalEnv={}", env.dump_env());
    }

    // Run stackless bytecode checks.
    run_stackless_bytecode_pipeline(
        &env,
        stackless_bytecode_check_pipeline(&options),
        &mut targets,
    );
    check_errors(&env, emitter, "stackless-bytecode checks failed")?;

    // We throw away the stackless bytecode generated so far, and go back to the
    // AST to perform other AST-transforming optimizations.
    // If we do such AST-transforming optimizations before the stackless bytecode
    // checks, it can result in missed and/or incorrect warnings.
    env_optimization_pipeline(&options).run(&mut env);
    check_errors(&env, emitter, "env optimization errors")?;

    // Regenerate stackless bytecode after env optimizations.
    let mut targets = run_stackless_bytecode_gen(&env);
    check_errors(&env, emitter, "stackless bytecode generation errors")?;

    // Run stackless bytecode optimization passes and other required passes.
    run_stackless_bytecode_pipeline(
        &env,
        stackless_bytecode_optimization_pipeline(&options),
        &mut targets,
    );
    check_errors(&env, emitter, "stackless-bytecode optimization errors")?;

    if options.experiment_on(Experiment::STOP_BEFORE_FILE_FORMAT) {
        std::process::exit(if env.has_warnings() { 1 } else { 0 })
    }

    // Generate the file format bytecode from the stackless bytecode.
    let modules_and_scripts = run_file_format_gen(&mut env, &targets);
    check_errors(&env, emitter, "assembling errors")?;

    if DEBUG {
        debug!(
            "File format bytecode:\n{}",
            disassemble_compiled_units(&modules_and_scripts)?
        );
    }

    // Run the bytecode verifier on the generated bytecode. We should never generate invalid bytecode,
    // so this check ensures we don't silently produce invalid bytecode.
    let annotated_units = annotate_units(modules_and_scripts);
    run_bytecode_verifier(&annotated_units, &mut env);
    check_errors(&env, emitter, "bytecode verification errors")?;

    // Finally mark this model to be generated by v2.
    env.set_compiler_v2(true);

    Ok((env, annotated_units))
}

/// Run move compiler and print errors to given writer for the purpose of analysis, like
/// e.g. the Move prover. After successful compilation attaches the generated bytecode
/// to the model.
pub fn run_move_compiler_for_analysis(
    error_writer: &mut impl WriteColor,
    mut options: Options,
) -> anyhow::Result<GlobalEnv> {
    options.whole_program = true; // will set `treat_everything_as_target`
    options = options.set_experiment(Experiment::SPEC_REWRITE, true);
    options = options.set_experiment(Experiment::ATTACH_COMPILED_MODULE, true);
    let mut emitter = options.error_emitter(error_writer);
    let (env, _units) = run_move_compiler(emitter.as_mut(), options)?;
    // Reset for subsequent analysis
    env.treat_everything_as_target(false);
    Ok(env)
}

/// Run the type checker and return the global env (with errors if encountered). The result
/// fails not on context checking errors, but possibly on i/o errors.
pub fn run_checker(options: Options) -> anyhow::Result<GlobalEnv> {
    info!("Type Checking");
    // Run the model builder, which performs context checking.
    let addrs = move_model::parse_addresses_from_options(options.named_address_mapping.clone())?;
    let mut env = move_model::run_model_builder_in_compiler_mode(
        PackageInfo {
            sources: options.sources.clone(),
            address_map: addrs.clone(),
        },
        PackageInfo {
            sources: options.sources_deps.clone(),
            address_map: addrs.clone(),
        },
        vec![PackageInfo {
            sources: options.dependencies.clone(),
            address_map: addrs.clone(),
        }],
        options.skip_attribute_checks,
        if !options.skip_attribute_checks && options.known_attributes.is_empty() {
            KnownAttribute::get_all_attribute_names()
        } else {
            &options.known_attributes
        },
        options.language_version.unwrap_or_default(),
        options.warn_deprecated,
        options.warn_of_deprecation_use_in_aptos_libs,
        options.compile_test_code,
        options.compile_verify_code,
    )?;
    // Store address aliases
    let map = addrs
        .into_iter()
        .map(|(s, a)| (env.symbol_pool().make(&s), a.into_inner()))
        .collect();
    env.set_address_alias_map(map);
    if options.whole_program {
        env.treat_everything_as_target(true);
    }
    // Store options in env, for later access
    env.set_extension(options);
    Ok(env)
}

/// Run the type checker, various required transforms (like inlining and lambda lifting),
/// and related additional checks, returning the global env (with errors if encountered).
/// The result fails not on checking errors, but possibly on i/o errors.
pub fn run_checker_and_rewriters(options: Options) -> anyhow::Result<GlobalEnv> {
    let whole_program = options.whole_program;
    let env_pipeline = env_check_and_transform_pipeline(&options);
    let mut env = run_checker(options)?;
    if !env.has_errors() {
        if whole_program {
            env.treat_everything_as_target(true)
        }
        env_pipeline.run(&mut env);
    }
    Ok(env)
}

/// Run a given stackless bytecode pipeline on the `targets`.
pub fn run_stackless_bytecode_pipeline(
    env: &GlobalEnv,
    pipeline: FunctionTargetPipeline,
    targets: &mut FunctionTargetsHolder,
) {
    let options = env.get_extension::<Options>().expect("options");
    if log_enabled!(Level::Debug) {
        // Dump bytecode, providing a name for the target derived from the first input file.
        let dump_base_name = options
            .sources
            .first()
            .and_then(|f| {
                Path::new(f)
                    .file_name()
                    .map(|f| f.to_string_lossy().as_ref().to_owned())
            })
            .unwrap_or_else(|| "dump".to_owned());
        pipeline.run_with_dump(
            env,
            targets,
            &dump_base_name,
            false,
            &pipeline::register_formatters,
            || !env.has_errors(),
        )
    } else {
        pipeline.run_with_hook(env, targets, |_| {}, |_, _, _| !env.has_errors())
    }
}

/// Run the stackless bytecode generator. For each function which is target of the
/// compilation, create an entry in the functions target holder which encapsulate info
/// like the generated bytecode.
pub fn run_stackless_bytecode_gen(env: &GlobalEnv) -> FunctionTargetsHolder {
    info!("Stackless bytecode Generation");
    let mut targets = FunctionTargetsHolder::default();
    let mut todo = BTreeSet::new();
    let mut done = BTreeSet::new();
    for module in env.get_modules() {
        if module.is_target() {
            for fun in module.get_functions() {
                let id = fun.get_qualified_id();
                // Skip inline functions because invoke and lambda are not supported in the current code generator
                if !fun.is_inline() {
                    todo.insert(id);
                }
            }
        }
    }
    while let Some(id) = todo.pop_first() {
        done.insert(id);
        let func_env = env.get_function(id);
        let data = bytecode_generator::generate_bytecode(env, id);
        targets.insert_target_data(&id, FunctionVariant::Baseline, data);
        for callee in func_env
            .get_used_functions()
            .expect("used functions available")
        {
            if !done.contains(callee) {
                todo.insert(*callee);
            }
        }
    }
    targets
}

/// Generate file format bytecode from the stackless bytecode targets.
pub fn run_file_format_gen(
    env: &mut GlobalEnv,
    targets: &FunctionTargetsHolder,
) -> Vec<CompiledUnit> {
    info!("File Format Generation");
    file_format_generator::generate_file_format(env, targets)
}

/// Constructs the env processor pipeline for various checks and transforms.
/// Note that these don't include AST transforms for optimizations.
pub fn env_check_and_transform_pipeline<'a, 'b>(options: &'a Options) -> EnvProcessorPipeline<'b> {
    let mut env_pipeline = EnvProcessorPipeline::<'b>::default();

    if options.experiment_on(Experiment::USAGE_CHECK) {
        env_pipeline.add(
            "unused checks",
            flow_insensitive_checkers::check_for_unused_vars_and_params,
        );
        env_pipeline.add(
            "type parameter check",
            function_checker::check_for_function_typed_parameters,
        );
    }

    if options.experiment_on(Experiment::RECURSIVE_TYPE_CHECK) {
        env_pipeline.add("check recursive struct definition", |env| {
            recursive_struct_checker::check_recursive_struct(env)
        });
        env_pipeline.add("check cyclic type instantiation", |env| {
            cyclic_instantiation_checker::check_cyclic_instantiations(env)
        });
    }

    if options.experiment_on(Experiment::UNUSED_STRUCT_PARAMS_CHECK) {
        env_pipeline.add("unused struct params check", |env| {
            unused_params_checker::unused_params_checker(env)
        });
    }

    if options.experiment_on(Experiment::ACCESS_CHECK) {
        env_pipeline.add(
            "access and use check before inlining",
            |env: &mut GlobalEnv| function_checker::check_access_and_use(env, true),
        );
    }

    let check_seqs_in_binops = !options
        .language_version
        .unwrap_or_default()
        .is_at_least(LanguageVersion::V2_0)
        && options.experiment_on(Experiment::SEQS_IN_BINOPS_CHECK);
    if check_seqs_in_binops {
        env_pipeline.add("binop side effect check", |env| {
            // This check should be done before inlining.
            seqs_in_binop_checker::checker(env)
        });
    }

    if options.experiment_on(Experiment::LINT_CHECKS) {
        // Perform all the model AST lint checks before inlining, to be closer "in form"
        // to the user code.
        env_pipeline.add("model AST lints", model_ast_lints::checker);
    }

    // The comparison rewriter is a new features in Aptos Move 2.2 and onwards
    let rewrite_cmp = options
        .language_version
        .unwrap_or_default()
        .is_at_least(LanguageVersion::V2_2)
        && options.experiment_on(Experiment::CMP_REWRITE);

    if rewrite_cmp {
        env_pipeline.add("rewrite comparison operations", |env| {
            // This rewrite is suggested to run before inlining to avoid repeated rewriting
            cmp_rewriter::rewrite(env);
        });
    }

    if options.experiment_on(Experiment::INLINING) {
        let rewriting_scope = if options.whole_program {
            RewritingScope::Everything
        } else {
            RewritingScope::CompilationTarget
        };
        let keep_inline_funs = options.experiment_on(Experiment::KEEP_INLINE_FUNS);
        let lift_inline_funs = options.experiment_on(Experiment::LIFT_INLINE_FUNS);
        env_pipeline.add("inlining", {
            move |env| {
                inliner::run_inlining(env, rewriting_scope, keep_inline_funs, lift_inline_funs)
            }
        });
    }

    if options.experiment_on(Experiment::ACCESS_CHECK) {
        env_pipeline.add(
            "access and use check after inlining",
            |env: &mut GlobalEnv| function_checker::check_access_and_use(env, false),
        );
    }

    if options.experiment_on(Experiment::ACQUIRES_CHECK) {
        env_pipeline.add("acquires check", |env| {
            acquires_checker::acquires_checker(env)
        });
    }

    if options
        .language_version
        .unwrap_or_default()
        .is_at_least(LanguageVersion::V2_2)
    {
        let include_inline_functions = options.experiment_on(Experiment::LAMBDA_LIFTING_INLINE);
        env_pipeline.add("lambda-lifting", move |env: &mut GlobalEnv| {
            lambda_lifter::lift_lambdas(
                LambdaLiftingOptions {
                    include_inline_functions,
                },
                env,
            )
        });
    }

    if options
        .language_version
        .unwrap_or_default()
        .is_at_least(LanguageVersion::V2_2)
    {
        env_pipeline.add("closure-checker", |env: &mut GlobalEnv| {
            closure_checker::check_closures(env)
        });
    }

    if options.experiment_on(Experiment::SPEC_CHECK) {
        env_pipeline.add("specification checker", |env| {
            let env: &GlobalEnv = env;
            spec_checker::run_spec_checker(env)
        });
    }

    if options.experiment_on(Experiment::SPEC_REWRITE) {
        // Same as above for spec-check.
        env_pipeline.add("specification rewriter", spec_rewriter::run_spec_rewriter);
    }

    env_pipeline
}

/// Constructs the env processor pipeline for various AST-transform optimizations.
pub fn env_optimization_pipeline<'a, 'b>(options: &'a Options) -> EnvProcessorPipeline<'b> {
    let mut env_pipeline = EnvProcessorPipeline::<'b>::default();

    if options.experiment_on(Experiment::AST_SIMPLIFY_FULL) {
        env_pipeline.add("simplifier with code elimination", {
            |env: &mut GlobalEnv| ast_simplifier::run_simplifier(env, true)
        });
    } else if options.experiment_on(Experiment::AST_SIMPLIFY) {
        env_pipeline.add("simplifier", {
            |env: &mut GlobalEnv| ast_simplifier::run_simplifier(env, false)
        });
    }

    env_pipeline
}

/// Returns the stackless bytecode processing pipeline for various checks.
pub fn stackless_bytecode_check_pipeline(options: &Options) -> FunctionTargetPipeline {
    let mut pipeline = FunctionTargetPipeline::default();

    // Preprocessing of the stackless bytecode. Many passes expect the absence of critical edges.
    if options.experiment_on(Experiment::SPLIT_CRITICAL_EDGES) {
        pipeline.add_processor(Box::new(SplitCriticalEdgesProcessor {}));
    }

    if options.experiment_on(Experiment::UNINITIALIZED_CHECK) {
        let keep_annotations = options.experiment_on(Experiment::KEEP_UNINIT_ANNOTATIONS);
        pipeline.add_processor(Box::new(UninitializedUseChecker { keep_annotations }));
    }

    if options.experiment_on(Experiment::UNUSED_ASSIGNMENT_CHECK) {
        // Unused assignment checker requires live variable annotations.
        pipeline.add_processor(Box::new(LiveVarAnalysisProcessor::new(false)));
        pipeline.add_processor(Box::new(UnusedAssignmentChecker {}));
    }

    // Reference safety checkers need live variable annotation.
    pipeline.add_processor(Box::new(LiveVarAnalysisProcessor::new(false)));
    if options.experiment_on(Experiment::REFERENCE_SAFETY_V3) {
        pipeline.add_processor(Box::new(
            reference_safety_processor_v3::ReferenceSafetyProcessor {},
        ));
    } else {
        // Reference check is always run, but the legacy processor decides internally
        // based on `Experiment::REFERENCE_SAFETY` whether to report errors.
        pipeline.add_processor(Box::new(
            reference_safety_processor_v2::ReferenceSafetyProcessor {},
        ));
    }

    if options.experiment_on(Experiment::ABILITY_CHECK) {
        pipeline.add_processor(Box::new(ExitStateAnalysisProcessor {}));
        pipeline.add_processor(Box::new(AbilityProcessor {}));
    }

    if options.experiment_on(Experiment::LINT_CHECKS) {
        // Some lint checks need live variable analysis.
        pipeline.add_processor(Box::new(LiveVarAnalysisProcessor::new(false)));
        pipeline.add_processor(Box::new(LintProcessor {}));
    }

    pipeline
}

/// Returns the stackless bytecode processing pipeline for optimization passes.
pub fn stackless_bytecode_optimization_pipeline(options: &Options) -> FunctionTargetPipeline {
    let mut pipeline = FunctionTargetPipeline::default();

    // Preprocessing of the stackless bytecode. Many passes expect the absence of critical edges.
    if options.experiment_on(Experiment::SPLIT_CRITICAL_EDGES) {
        pipeline.add_processor(Box::new(SplitCriticalEdgesProcessor {}));
    }

    // Reference safety checkers need live variable annotation.
    pipeline.add_processor(Box::new(LiveVarAnalysisProcessor::new(false)));
    if options.experiment_on(Experiment::REFERENCE_SAFETY_V3) {
        pipeline.add_processor(Box::new(
            reference_safety_processor_v3::ReferenceSafetyProcessor {},
        ));
    } else {
        // Reference check is always run, but the legacy processor decides internally
        // based on `Experiment::REFERENCE_SAFETY` whether to report errors.
        pipeline.add_processor(Box::new(
            reference_safety_processor_v2::ReferenceSafetyProcessor {},
        ));
    }

    // Assignment kind (copy or move) and drops need to inferred for the later code generation.
    if options.experiment_on(Experiment::ABILITY_CHECK) {
        pipeline.add_processor(Box::new(ExitStateAnalysisProcessor {}));
        pipeline.add_processor(Box::new(AbilityProcessor {}));
    }

    if options.experiment_on(Experiment::CFG_SIMPLIFICATION) {
        pipeline.add_processor(Box::new(ControlFlowGraphSimplifier {}));
        if options.experiment_on(Experiment::SPLIT_CRITICAL_EDGES) {
            // Currently, CFG simplification can again introduce critical edges, so
            // remove them. Notice that absence of critical edges is relevant
            // for the livevar processor, which is used frequently below.
            pipeline.add_processor(Box::new(SplitCriticalEdgesProcessor {}));
        }
    }

    if options.experiment_on(Experiment::DEAD_CODE_ELIMINATION) {
        pipeline.add_processor(Box::new(UnreachableCodeProcessor {}));
        pipeline.add_processor(Box::new(UnreachableCodeRemover {}));
        pipeline.add_processor(Box::new(LiveVarAnalysisProcessor::new(true)));
        pipeline.add_processor(Box::new(DeadStoreElimination::new(true)));
    }

    if options.experiment_on(Experiment::VARIABLE_COALESCING) {
        // Live var analysis is needed by variable coalescing.
        pipeline.add_processor(Box::new(LiveVarAnalysisProcessor::new(false)));
        if options.experiment_on(Experiment::VARIABLE_COALESCING_ANNOTATE) {
            pipeline.add_processor(Box::new(VariableCoalescing::annotate_only()));
        }
        pipeline.add_processor(Box::new(VariableCoalescing::transform_only()));
    }

    if options.experiment_on(Experiment::DEAD_CODE_ELIMINATION) {
        pipeline.add_processor(Box::new(LiveVarAnalysisProcessor::new(true)));
        pipeline.add_processor(Box::new(DeadStoreElimination::new(false)));
    }

    // Run live var analysis again because it could be invalidated by previous pipeline steps,
    // but it is needed by file format generator.
    // There should be no stackless bytecode changes after this point.
    pipeline.add_processor(Box::new(LiveVarAnalysisProcessor::new(true)));

    if options.experiment_on(Experiment::FLUSH_WRITES_OPTIMIZATION) {
        // This processor only adds annotations, does not transform the bytecode.
        pipeline.add_processor(Box::new(FlushWritesProcessor {}));
    }

    pipeline
}

/// Disassemble the given compiled units and return the disassembled code as a string.
pub fn disassemble_compiled_units(units: &[CompiledUnit]) -> anyhow::Result<String> {
    let disassembled_units: anyhow::Result<Vec<_>> = units
        .iter()
        .map(|unit| Disassembler::from_unit(unit).disassemble())
        .collect();
    Ok(disassembled_units?.concat())
}

/// Run the bytecode verifier on the given compiled units and add any diagnostics to the global env.
pub fn run_bytecode_verifier(units: &[AnnotatedCompiledUnit], env: &mut GlobalEnv) -> bool {
    let mut errors = false;
    for unit in units {
        match unit {
            AnnotatedCompiledUnit::Module(AnnotatedCompiledModule {
                loc,
                named_module:
                    NamedCompiledModule {
                        module, source_map, ..
                    },
                ..
            }) => {
                if let Err(e) = move_bytecode_verifier::verify_module(module) {
                    report_bytecode_verification_error(env, loc, source_map, &e);
                    errors = true
                }
            },
            AnnotatedCompiledUnit::Script(AnnotatedCompiledScript {
                loc,
                named_script:
                    NamedCompiledScript {
                        script, source_map, ..
                    },
                ..
            }) => {
                if let Err(e) = move_bytecode_verifier::verify_script(script) {
                    report_bytecode_verification_error(env, loc, source_map, &e);
                    errors = true
                }
            },
        }
    }
    !errors
}

fn report_bytecode_verification_error(
    env: &GlobalEnv,
    module_ir_loc: &MoveIrLoc,
    source_map: &SourceMap,
    e: &VMError,
) {
    let mut precise_loc = true;
    let loc = &get_vm_error_loc(env, source_map, e).unwrap_or_else(|| {
        precise_loc = false;
        env.to_loc(module_ir_loc)
    });
    if e.status_type() != StatusType::Verification {
        env.diag_with_notes(
            Severity::Bug,
            loc,
            &format!(
                "unexpected error returned from bytecode verification:\n{:#?}",
                e
            ),
            vec![COMPILER_BUG_REPORT_MSG.to_string()],
        )
    } else {
        let debug_info = if command_line::get_move_compiler_backtrace_from_env() {
            format!("\n{:#?}", e)
        } else {
            format!(
                "\nError message: {}",
                e.message().cloned().unwrap_or_else(|| "none".to_string())
            )
        };
        env.diag_with_notes(
            Severity::Bug,
            loc,
            &format!(
                "bytecode verification failed with unexpected status code `{:?}`:{}",
                e.major_status(),
                debug_info
            ),
            vec![COMPILER_BUG_REPORT_MSG.to_string()],
        )
    }
}

/// Gets the location associated with the VM error, if available.
fn get_vm_error_loc(env: &GlobalEnv, source_map: &SourceMap, e: &VMError) -> Option<Loc> {
    e.offsets().first().and_then(|(fdef_idx, offset)| {
        source_map
            .get_code_location(*fdef_idx, *offset)
            .ok()
            .map(|l| env.to_loc(&l))
    })
}

/// Report any diags in the env to the writer and fail if there are errors.
pub fn check_errors<E>(env: &GlobalEnv, emitter: &mut E, msg: &str) -> anyhow::Result<()>
where
    E: Emitter + ?Sized,
{
    let options = env.get_extension::<Options>().unwrap_or_default();

    emitter.report_diag(env, options.report_severity());
    emitter.check_diag(env, options.report_severity(), msg)
}

/// Annotate the given compiled units.
pub fn annotate_units(units: Vec<CompiledUnit>) -> Vec<AnnotatedCompiledUnit> {
    units
        .into_iter()
        .map(|u| match u {
            CompiledUnit::Module(named_module) => {
                let loc = named_module.source_map.definition_location;
                AnnotatedCompiledUnit::Module(AnnotatedCompiledModule {
                    loc,
                    module_name_loc: loc,
                    address_name: None,
                    named_module,
                })
            },
            CompiledUnit::Script(named_script) => {
                AnnotatedCompiledUnit::Script(AnnotatedCompiledScript {
                    loc: named_script.source_map.definition_location,
                    named_script,
                    function_info: FunctionInfo {
                        spec_info: Default::default(),
                    },
                })
            },
        })
        .collect()
}

/// Computes the `FilesSourceText` from the global environment, which maps IR loc file hashes
/// into files and sources. This value is used for the package system only.
pub fn make_files_source_text(env: &GlobalEnv) -> FilesSourceText {
    let mut result = FilesSourceText::new();
    for fid in env.get_source_file_ids() {
        if let Some(hash) = env.get_file_hash(fid) {
            let file_name = Symbol::from(env.get_file(fid).to_string_lossy().to_string());
            let file_content = env.get_file_source(fid).to_owned();
            result.insert(hash, (file_name, file_content));
        }
    }
    result
}
