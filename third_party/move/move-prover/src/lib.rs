// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::cli::Options;
use anyhow::anyhow;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream, WriteColor};
use itertools::Itertools;
#[allow(unused_imports)]
use log::{debug, info, warn};
use log::{log_enabled, Level};
use move_compiler_v2::Experiment;
use move_model::{
    code_writer::CodeWriter,
    metadata::LATEST_STABLE_COMPILER_VERSION_VALUE,
    model::{FunId, FunctionEnv, GlobalEnv, ModuleId, QualifiedId},
    pragmas::{TIMEOUT_PRAGMA, VERIFY_DURATION_ESTIMATE_PRAGMA},
};
use move_prover_boogie_backend::{
    add_prelude,
    boogie_wrapper::{run_boogies_parallel, BoogieWrapper},
    bytecode_translator::{BoogieTranslator, VerifyTargetSelector},
    options::VerifyGranularity,
};
use move_prover_bytecode_pipeline::{
    number_operation::GlobalNumberOperationState, pipeline_factory,
};
use move_stackless_bytecode::function_target_pipeline::{FunctionTargetsHolder, FunctionVariant};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    time::{Duration, Instant},
};

pub mod cli;
pub mod inference;
pub mod package_prove;

// =================================================================================================
// Prover API

pub fn run_move_prover_errors_to_stderr(options: Options) -> anyhow::Result<()> {
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    run_move_prover_v2(&mut error_writer, options, vec![])
}

pub fn run_move_prover_v2<W: WriteColor>(
    error_writer: &mut W,
    options: Options,
    mut experiments: Vec<String>,
) -> anyhow::Result<()> {
    let now = Instant::now();
    if options.inference.inference || !options.prover.no_infer_lambda_specs {
        // Lambda spec inference benefits from pure-spec-fun rewriting too: lambda
        // bodies that call pure user functions then inference cleanly to
        // `result == helper(args)` instead of `result == result_of<helper>(args)`.
        experiments.push(Experiment::SPEC_REWRITE_PURE_FUNS.to_string());
    }
    let mut env = create_move_prover_v2_model(error_writer, options.clone(), experiments)?;
    if options.inference.inference {
        inference::run_spec_inference_with_model(&mut env, error_writer, options, now)
    } else {
        run_move_prover_with_model_v2(&mut env, error_writer, options, now)
    }
}

/// Like `run_move_prover_v2` for inference, but also returns a bytecode dump
/// with WP annotations for debugging test baselines.
pub fn run_inference_with_bytecode_dump<W: WriteColor>(
    error_writer: &mut W,
    options: Options,
    mut experiments: Vec<String>,
) -> anyhow::Result<String> {
    let now = Instant::now();
    experiments.push(Experiment::SPEC_REWRITE_PURE_FUNS.to_string());
    let mut env = create_move_prover_v2_model(error_writer, options.clone(), experiments)?;
    inference::run_spec_inference_with_model_and_dump(&mut env, error_writer, options, now)
}

pub fn create_move_prover_v2_model<W: WriteColor>(
    error_writer: &mut W,
    options: Options,
    experiments: Vec<String>,
) -> anyhow::Result<GlobalEnv> {
    let compiler_options = move_compiler_v2::Options {
        dependencies: options.move_deps,
        named_address_mapping: options.move_named_address_values,
        output_dir: options.output_path,
        language_version: options.language_version,
        compiler_version: Some(LATEST_STABLE_COMPILER_VERSION_VALUE),
        skip_attribute_checks: true,
        known_attributes: Default::default(),
        testing: options.backend.stable_test_output,
        experiments,
        experiment_cache: Default::default(),
        sources: options.move_sources,
        sources_deps: vec![],
        whole_program: false,
        compile_test_code: false,
        compile_verify_code: true,
        external_checks: vec![],
        print_errors: true,
    };

    move_compiler_v2::run_move_compiler_for_analysis(error_writer, compiler_options)
}

/// Create the initial number operation state for each function and struct
pub fn create_init_num_operation_state(env: &GlobalEnv) {
    let mut global_state: GlobalNumberOperationState = Default::default();
    for module_env in env.get_modules() {
        for struct_env in module_env.get_structs() {
            global_state.create_initial_struct_oper_state(&struct_env);
        }
        for fun_env in module_env.get_functions() {
            if !fun_env.is_not_prover_target() {
                global_state.create_initial_func_oper_state(&fun_env);
            }
        }
    }
    env.set_extension(global_state);
}

pub fn run_move_prover_with_model_v2<W: WriteColor>(
    env: &mut GlobalEnv,
    error_writer: &mut W,
    mut options: Options,
    start_time: Instant,
) -> anyhow::Result<()> {
    let build_duration = start_time.elapsed();
    check_errors(
        env,
        &options,
        error_writer,
        "exiting with model building errors",
    )?;

    // Add the prover options as an extension to the environment, so they can be accessed
    // from there.
    env.set_extension(options.prover.clone());

    // Populate initial number operation state for each function and struct based on the pragma
    create_init_num_operation_state(env);

    // Check correct backend versions.
    options.backend.check_tool_versions()?;

    // Create and process bytecode
    let now = Instant::now();
    let targets = create_and_process_bytecode(&options, env);
    let trafo_duration = now.elapsed();
    check_errors(
        env,
        &options,
        error_writer,
        "exiting with bytecode transformation errors",
    )?;

    // Reject conflicting partition flags up front. `--shards N` only makes sense
    // under the default `shard` granularity; combining it with the finer
    // partitions would silently dilute one or the other.
    if !matches!(options.backend.granularity, VerifyGranularity::Shard)
        && options.backend.shards > 1
    {
        return Err(anyhow!(
            "`--shards > 1` only applies to `--granularity shard`. \
             Use one of `--granularity shard --shards N`, `--granularity module`, \
             or `--granularity vc`."
        ));
    }

    let mut gen_durations = vec![];
    let mut verify_durations = vec![];
    let output_base_file = options.output_path.clone();
    match options.backend.granularity {
        VerifyGranularity::Shard => drive_sharded(
            env,
            &mut options,
            &targets,
            &output_base_file,
            error_writer,
            &mut gen_durations,
            &mut verify_durations,
        )?,
        VerifyGranularity::Module => drive_per_module(
            env,
            &mut options,
            &targets,
            &output_base_file,
            error_writer,
            &mut gen_durations,
            &mut verify_durations,
        )?,
        VerifyGranularity::Vc => drive_per_vc(
            env,
            &mut options,
            &targets,
            &output_base_file,
            error_writer,
            &mut gen_durations,
            &mut verify_durations,
        )?,
    }
    options.output_path = output_base_file;
    // Report durations.
    let dur_list = |ds: &[Duration]| {
        ds.iter()
            .map(|d| format!("{:.2}", d.as_secs_f64()))
            .join("/")
    };
    info!(
        "{:.2}s build, {:.2}s trafo, {}s gen, {}s verify, total {:.2}s",
        build_duration.as_secs_f64(),
        trafo_duration.as_secs_f64(),
        dur_list(&gen_durations),
        dur_list(&verify_durations),
        build_duration.as_secs_f64()
            + trafo_duration.as_secs_f64()
            + gen_durations.iter().sum::<Duration>().as_secs_f64()
            + verify_durations.iter().sum::<Duration>().as_secs_f64()
    );
    check_errors(
        env,
        &options,
        error_writer,
        "exiting with verification errors",
    )
}

pub fn check_errors<W: WriteColor>(
    env: &GlobalEnv,
    options: &Options,
    error_writer: &mut W,
    msg: &'static str,
) -> anyhow::Result<()> {
    env.report_diag(error_writer, options.prover.report_severity);
    if env.has_errors() {
        Err(anyhow!(msg))
    } else {
        Ok(())
    }
}

/// Generate Boogie for the verify targets selected by `selector`.
pub fn generate_boogie_with_selector(
    env: &GlobalEnv,
    options: &Options,
    selector: VerifyTargetSelector,
    targets: &FunctionTargetsHolder,
) -> anyhow::Result<CodeWriter> {
    let writer = CodeWriter::new(env.internal_loc());
    add_prelude(env, &options.backend, &writer)?;
    let mut translator = BoogieTranslator::new(env, &options.backend, selector, targets, &writer);
    translator.translate();
    Ok(writer)
}

/// Back-compat wrapper for callers that pass `Option<usize>` shard index. Prefer
/// `generate_boogie_with_selector` in new code.
pub fn generate_boogie(
    env: &GlobalEnv,
    options: &Options,
    shard: Option<usize>,
    targets: &FunctionTargetsHolder,
) -> anyhow::Result<CodeWriter> {
    let selector = match shard {
        None => VerifyTargetSelector::All,
        Some(idx) => VerifyTargetSelector::Shard {
            idx,
            total: options.backend.shards,
        },
    };
    generate_boogie_with_selector(env, options, selector, targets)
}

/// Mirrors the verify-target predicate inside `BoogieTranslator::is_verified`:
/// function passes the structural filter and its `verify_duration_estimate`
/// pragma (if present) fits inside the effective timeout. Used by the
/// enumeration helpers so per-module / per-VC drivers don't generate `.bpl`
/// files for targets the translator would silently skip.
fn function_passes_verify_filter(fun_env: &FunctionEnv, options: &cli::Options) -> bool {
    if fun_env.is_native_or_intrinsic() || fun_env.is_test_only() || fun_env.is_not_prover_target()
    {
        return false;
    }
    if let Some(estimate_timeout) = fun_env.get_num_pragma(VERIFY_DURATION_ESTIMATE_PRAGMA) {
        let timeout = fun_env
            .get_num_pragma(TIMEOUT_PRAGMA)
            .unwrap_or(options.backend.vc_timeout);
        estimate_timeout <= timeout
    } else {
        true
    }
}

/// Enumerate every (function-id, FunctionVariant::Verification(_)) pair the prover
/// would emit `$verify` procedures for under the default selector. Source of truth
/// for the per-VC outer loop. Mirrors `bytecode_translator::translate`'s iteration —
/// walks **every** module (not only `is_target()` ones, since
/// `VerificationAnalysisProcessor` can mark functions in non-target modules as
/// verified when a target-module global invariant must be checked there), filters
/// functions by the same `is_native_or_intrinsic` / `is_test_only` /
/// `is_not_prover_target` rules, and applies the `verify_duration_estimate` /
/// `timeout` pragma check so we don't codegen `.bpl` files whose `$verify`
/// procedure would be filtered out at translation time.
///
/// V1 simplification: type-instance variants (`VerificationFlavor::Instantiated(_)`)
/// from `mono_info.funs` are NOT enumerated separately; each base verify target gets
/// one `.bpl`. Per-VC mode therefore implicitly behaves as if `--skip-instance-check`
/// were set.
pub fn enumerate_verify_targets(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    options: &cli::Options,
) -> Vec<(QualifiedId<FunId>, FunctionVariant)> {
    let mut out = Vec::new();
    for module_env in env.get_modules() {
        for fun_env in module_env.get_functions() {
            if !function_passes_verify_filter(&fun_env, options) {
                continue;
            }
            for (variant, _target) in targets.get_targets(&fun_env) {
                if !variant.is_verified() {
                    continue;
                }
                out.push((fun_env.get_qualified_id(), variant));
            }
        }
    }
    out
}

/// Enumerate every module that contains at least one verify target. Same
/// inclusion rules as `enumerate_verify_targets`: walks every module (so
/// non-target modules containing verify variants for target-module invariants
/// are not dropped) and respects the `verify_duration_estimate` pragma.
/// Stable iteration order (source-declaration order, matching `env.get_modules`).
pub fn enumerate_modules_with_verify_targets(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    options: &cli::Options,
) -> Vec<ModuleId> {
    let mut out = Vec::new();
    for module_env in env.get_modules() {
        let has_verify_target = module_env.get_functions().any(|fun_env| {
            if !function_passes_verify_filter(&fun_env, options) {
                return false;
            }
            targets
                .get_targets(&fun_env)
                .iter()
                .any(|(variant, _)| variant.is_verified())
        });
        if has_verify_target {
            out.push(module_env.get_id());
        }
    }
    out
}

/// Sanitize a string for use as a portion of a filename. Replaces characters that
/// commonly cause shell / path issues with `_`. Stable, deterministic mapping.
pub fn sanitize_for_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ':' | '<' | '>' | ',' | ' ' | '/' | '\\' | '"' | '\'' | '#' | '|' | '?' | '*' => '_',
            other => other,
        })
        .collect()
}

/// Build the per-VC `.bpl` output path: `<base_no_ext>.vc_<sanitized>_<variant>.bpl`.
fn per_vc_output_path(
    env: &GlobalEnv,
    output_base: &str,
    qid: QualifiedId<FunId>,
    variant: &FunctionVariant,
) -> String {
    let full_name = env.get_function(qid).get_full_name_with_address();
    let stem = sanitize_for_filename(&format!("{}_{}", full_name, variant));
    Path::new(output_base)
        .with_extension(format!("vc_{}.bpl", stem))
        .to_string_lossy()
        .into_owned()
}

/// Build the per-module `.bpl` output path: `<base_no_ext>.module_<sanitized>.bpl`.
fn per_module_output_path(env: &GlobalEnv, output_base: &str, module_id: ModuleId) -> String {
    let stem = sanitize_for_filename(&env.get_module(module_id).get_full_name_str());
    Path::new(output_base)
        .with_extension(format!("module_{}.bpl", stem))
        .to_string_lossy()
        .into_owned()
}

/// Sharded (or unsharded single-`.bpl`) driver. Preserves existing behavior:
/// sequential one-`.bpl`-per-shard with Boogie's intra-process parallelism on
/// procedures inside each `.bpl`.
fn drive_sharded<W: WriteColor>(
    env: &GlobalEnv,
    options: &mut Options,
    targets: &FunctionTargetsHolder,
    output_base_file: &str,
    error_writer: &mut W,
    gen_durations: &mut Vec<Duration>,
    verify_durations: &mut Vec<Duration>,
) -> anyhow::Result<()> {
    let has_shards = options.backend.shards > 1;
    for shard in 0..options.backend.shards {
        if has_shards {
            options.output_path = Path::new(output_base_file)
                .with_extension(format!("shard_{}.bpl", shard + 1))
                .to_string_lossy()
                .to_string();
        }
        let selector = if has_shards {
            VerifyTargetSelector::Shard {
                idx: shard,
                total: options.backend.shards,
            }
        } else {
            VerifyTargetSelector::All
        };
        let now = Instant::now();
        let code_writer = generate_boogie_with_selector(env, options, selector, targets)?;
        gen_durations.push(now.elapsed());
        check_errors(
            env,
            options,
            error_writer,
            "exiting with condition generation errors",
        )?;
        let now = Instant::now();
        verify_boogie(env, options, targets, code_writer)?;
        verify_durations.push(now.elapsed());
    }
    Ok(())
}

/// Per-module driver: one `.bpl` per source module with verify targets. Each
/// module gets its own pipeline rerun via `restrict_primary_targets_to`, so the
/// `.bpl` matches `move prove -f <module>` content. `check_errors` is deferred
/// to after all reruns for cross-rerun diagnostic dedup.
fn drive_per_module<W: WriteColor>(
    env: &GlobalEnv,
    options: &mut Options,
    upstream_targets: &FunctionTargetsHolder,
    output_base_file: &str,
    error_writer: &mut W,
    gen_durations: &mut Vec<Duration>,
    verify_durations: &mut Vec<Duration>,
) -> anyhow::Result<()> {
    let module_list = enumerate_modules_with_verify_targets(env, upstream_targets, options);
    let parallelism = options.backend.proc_cores.max(1);
    info!(
        "per-module mode: {} modules, parallelism={}",
        module_list.len(),
        parallelism
    );

    // Phase 1: per-module pipeline rerun + codegen. Restrict primary targets
    // only; leaving `verify_scope = All` keeps Rule 3 in
    // `VerificationAnalysisProcessor` marking cross-module functions that
    // modify in-scope invariants — those drive part of the closure walk that
    // populates `MonoInfo`.
    let mut bpl_paths: Vec<String> = Vec::with_capacity(module_list.len());
    let mut per_module_targets: Vec<FunctionTargetsHolder> = Vec::with_capacity(module_list.len());
    for module_id in &module_list {
        let module_env = env.get_module(*module_id);
        let module_file_id = module_env.get_loc().file_id();
        options.output_path = per_module_output_path(env, output_base_file, *module_id);
        env.restrict_primary_targets_to(Some(BTreeSet::from([module_file_id])));

        let now = Instant::now();
        let targets = create_and_process_bytecode(options, env);
        let selector = VerifyTargetSelector::Module {
            module_id: *module_id,
        };
        let code_writer = generate_boogie_with_selector(env, options, selector, &targets)?;
        gen_durations.push(now.elapsed());
        code_writer.process_result(|result| fs::write(&options.output_path, result))?;
        bpl_paths.push(options.output_path.clone());
        per_module_targets.push(targets);
    }

    // Restore the persistent primary-target set so downstream code (or a
    // subsequent caller of the prover with the same env) sees the original
    // configuration.
    env.restrict_primary_targets_to(None);

    // Single deferred error check. `report_diag`'s `shown` fingerprint set
    // dedupes cross-rerun duplicates of scope-independent diagnostics in one
    // pass over the accumulated diag list.
    check_errors(
        env,
        options,
        error_writer,
        "exiting with bytecode transformation errors",
    )?;

    if options.prover.generate_only {
        return Ok(());
    }

    // Phase 2: parallel verification across modules.
    let now = Instant::now();
    let raw_results = run_boogies_parallel(&options.backend, &bpl_paths, parallelism);
    verify_durations.push(now.elapsed());

    // Analyze each captured result in stable input order against the
    // FunctionTargetsHolder that produced its `.bpl`. Don't short-circuit on
    // the first hard error — one `.bpl` failing shouldn't strand the rest.
    let mut first_err: Option<anyhow::Error> = None;
    for (i, (path, raw)) in bpl_paths.iter().zip(raw_results.into_iter()).enumerate() {
        let wrapper = BoogieWrapper {
            env,
            targets: &per_module_targets[i],
            writer: &CodeWriter::new(env.internal_loc()),
            options: &options.backend,
        };
        if let Err(e) = wrapper.analyze_subprocess_output(path, raw) {
            if first_err.is_none() {
                first_err = Some(e);
            }
        }
        if !options.backend.keep_artifacts {
            std::fs::remove_file(path).unwrap_or_default();
        }
    }
    if let Some(e) = first_err {
        Err(e)
    } else {
        Ok(())
    }
}

/// Per-VC driver: one `.bpl` per verify target. Outer loop reruns the pipeline
/// per module (same narrowing as `drive_per_module`); inner loop slices the
/// module's targets via `VerifyTargetSelector::Single`. All VCs from one module
/// share that module's pipeline result.
fn drive_per_vc<W: WriteColor>(
    env: &GlobalEnv,
    options: &mut Options,
    upstream_targets: &FunctionTargetsHolder,
    output_base_file: &str,
    error_writer: &mut W,
    gen_durations: &mut Vec<Duration>,
    verify_durations: &mut Vec<Duration>,
) -> anyhow::Result<()> {
    let module_list = enumerate_modules_with_verify_targets(env, upstream_targets, options);
    let parallelism = options.backend.proc_cores.max(1);

    // Phase 1: per-module pipeline rerun (outer), per-VC codegen slice (inner).
    let mut bpl_paths: Vec<String> = Vec::new();
    let mut per_bpl_targets: Vec<std::rc::Rc<FunctionTargetsHolder>> = Vec::new();
    for module_id in &module_list {
        let module_env = env.get_module(*module_id);
        let module_file_id = module_env.get_loc().file_id();
        env.restrict_primary_targets_to(Some(BTreeSet::from([module_file_id])));

        let module_targets = std::rc::Rc::new(create_and_process_bytecode(options, env));
        let vc_list = enumerate_verify_targets_in_module(env, &module_targets, options, *module_id);
        for (qid, variant) in &vc_list {
            options.output_path = per_vc_output_path(env, output_base_file, *qid, variant);
            let selector = VerifyTargetSelector::Single {
                qid: *qid,
                variant: variant.clone(),
            };
            let now = Instant::now();
            let code_writer =
                generate_boogie_with_selector(env, options, selector, &module_targets)?;
            gen_durations.push(now.elapsed());
            code_writer.process_result(|result| fs::write(&options.output_path, result))?;
            bpl_paths.push(options.output_path.clone());
            per_bpl_targets.push(std::rc::Rc::clone(&module_targets));
        }
    }
    info!(
        "per-VC mode: {} verify targets, parallelism={}",
        bpl_paths.len(),
        parallelism
    );

    env.restrict_primary_targets_to(None);

    check_errors(
        env,
        options,
        error_writer,
        "exiting with bytecode transformation errors",
    )?;

    if options.prover.generate_only {
        return Ok(());
    }

    // Phase 2: parallel verification.
    let now = Instant::now();
    let raw_results = run_boogies_parallel(&options.backend, &bpl_paths, parallelism);
    verify_durations.push(now.elapsed());

    let mut first_err: Option<anyhow::Error> = None;
    for (i, (path, raw)) in bpl_paths.iter().zip(raw_results.into_iter()).enumerate() {
        let wrapper = BoogieWrapper {
            env,
            targets: &per_bpl_targets[i],
            writer: &CodeWriter::new(env.internal_loc()),
            options: &options.backend,
        };
        if let Err(e) = wrapper.analyze_subprocess_output(path, raw) {
            if first_err.is_none() {
                first_err = Some(e);
            }
        }
        if !options.backend.keep_artifacts {
            std::fs::remove_file(path).unwrap_or_default();
        }
    }
    if let Some(e) = first_err {
        Err(e)
    } else {
        Ok(())
    }
}

/// Like `enumerate_verify_targets` but restricted to one module. Used by the
/// per-VC driver to slice a per-module pipeline result into per-VC `.bpl`s.
fn enumerate_verify_targets_in_module(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
    options: &cli::Options,
    module_id: ModuleId,
) -> Vec<(QualifiedId<FunId>, FunctionVariant)> {
    let mut out = Vec::new();
    let module_env = env.get_module(module_id);
    for fun_env in module_env.get_functions() {
        if !function_passes_verify_filter(&fun_env, options) {
            continue;
        }
        for (variant, _target) in targets.get_targets(&fun_env) {
            if !variant.is_verified() {
                continue;
            }
            out.push((fun_env.get_qualified_id(), variant));
        }
    }
    out
}

pub fn verify_boogie(
    env: &GlobalEnv,
    options: &Options,
    targets: &FunctionTargetsHolder,
    writer: CodeWriter,
) -> anyhow::Result<()> {
    let output_existed = std::path::Path::new(&options.output_path).exists();
    debug!("writing boogie to `{}`", &options.output_path);
    writer.process_result(|result| fs::write(&options.output_path, result))?;
    if !options.prover.generate_only {
        let boogie = BoogieWrapper {
            env,
            targets,
            writer: &writer,
            options: &options.backend,
        };
        boogie.call_boogie_and_verify_output(&options.output_path)?;
        if !output_existed && !options.backend.keep_artifacts {
            std::fs::remove_file(&options.output_path).unwrap_or_default();
        }
    }
    Ok(())
}

/// Create bytecode and process it.
pub fn create_and_process_bytecode(options: &Options, env: &GlobalEnv) -> FunctionTargetsHolder {
    let mut targets = FunctionTargetsHolder::default();
    let output_dir = Path::new(&options.output_path)
        .parent()
        .expect("expect the parent directory of the output path to exist");
    let output_prefix = options.move_sources.first().map_or("bytecode", |s| {
        Path::new(s).file_name().unwrap().to_str().unwrap()
    });

    // Add function targets for all functions in the environment.
    for module_env in env.get_modules() {
        if module_env.is_target() {
            info!("preparing module {}", module_env.get_full_name_str());
        }
        if options.prover.dump_bytecode {
            if let Some(out) = module_env.disassemble() {
                debug!("disassembled bytecode:\n{}", out);
            }
        }
        for func_env in module_env.get_functions() {
            if func_env.is_struct_api() {
                // Struct API wrappers have no user-written specs; skip them to avoid
                // spurious invariant failures from DataInvariantInstrumentationProcessor.
                continue;
            }
            targets.add_target(&func_env)
        }
    }

    // Create processing pipeline and run it.
    let pipeline = if options.experimental_pipeline {
        pipeline_factory::experimental_pipeline()
    } else {
        pipeline_factory::default_pipeline_with_options(&options.prover)
    };

    if log_enabled!(Level::Debug) && options.prover.dump_bytecode {
        let dump_file_base = output_dir
            .join(output_prefix)
            .into_os_string()
            .into_string()
            .unwrap();
        pipeline.run_with_dump(
            env,
            &mut targets,
            &dump_file_base,
            options.prover.dump_cfg,
            &|target| target.register_annotation_formatters_for_test(),
            || true,
        )
    } else {
        pipeline.run(env, &mut targets);
    }

    targets
}
