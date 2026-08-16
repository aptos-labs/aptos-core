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
    code_writer::CodeWriter, metadata::LATEST_STABLE_COMPILER_VERSION_VALUE, model::GlobalEnv,
};
use move_prover_boogie_backend::{
    add_prelude,
    boogie_wrapper::{BoogieRunStatus, BoogieWrapper},
    bytecode_translator::BoogieTranslator,
};
use move_prover_bytecode_pipeline::{
    mono_analysis::{self, MonoAnalysisProcessor, VerificationRoot},
    number_operation::GlobalNumberOperationState,
    pipeline_factory,
};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct VerificationTiming {
    pub function: String,
    pub verification_condition: String,
    pub duration: Duration,
    pub status: BoogieRunStatus,
}

struct BoogieJob {
    root: VerificationRoot,
    path: String,
    writer: CodeWriter,
    output_existed: bool,
    process_timeout: u64,
    seed_handoff_after: Option<Duration>,
    process_deadline: Option<Instant>,
}

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
    if options.inference.inference {
        // Spec inference benefits from pure-spec-fun rewriting: bodies that call
        // pure user functions then infer cleanly to `result == helper(args)`
        // instead of `result == result_of<helper>(args)`. In verify mode the
        // experiment stays off: lambda spec inference for lifted lambdas works
        // without it (pure callees are summarized via `result_of` instead), and
        // deriving spec functions for all pure Move functions destabilizes the
        // solver on spec-function specializations of accumulating HOFs.
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
    options: Options,
    start_time: Instant,
) -> anyhow::Result<()> {
    run_move_prover_with_model_v2_internal(env, error_writer, options, start_time, true)?;
    Ok(())
}

pub fn benchmark_move_prover_with_model_v2<W: WriteColor>(
    env: &mut GlobalEnv,
    error_writer: &mut W,
    mut options: Options,
) -> anyhow::Result<Vec<VerificationTiming>> {
    options.backend.proc_cores = 1;
    run_move_prover_with_model_v2_internal(env, error_writer, options, Instant::now(), false)
}

fn run_move_prover_with_model_v2_internal<W: WriteColor>(
    env: &mut GlobalEnv,
    error_writer: &mut W,
    options: Options,
    start_time: Instant,
    fail_on_verification_errors: bool,
) -> anyhow::Result<Vec<VerificationTiming>> {
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

    let mut gen_durations = vec![];
    let mut verify_durations = vec![];
    let mut timings = vec![];
    let (output_base_file, _temporary_boogie_dir) = verification_output_base(&options)?;
    let package_mono_info = mono_analysis::get_info(env);
    let roots = BoogieTranslator::verification_roots(env, &options.backend, &targets);
    info!("{} verification roots", roots.len());
    let root_count = roots.len();
    let package_deadline = if options.backend.package_timeout_secs == 0 {
        None
    } else {
        start_time.checked_add(Duration::from_secs(options.backend.package_timeout_secs))
    };
    let initial_error_count = env.error_count();
    if options.prover.generate_only {
        for (root_index, root) in roots.into_iter().enumerate() {
            let path = verification_output_path(&output_base_file, root_count, root_index);
            let (job, duration) = generate_boogie_job(
                env,
                &options,
                &targets,
                package_mono_info.as_ref(),
                path,
                root,
                package_deadline,
            )?;
            gen_durations.push(duration);
            if package_error_limit_reached(env, &options, initial_error_count) {
                break;
            }
            drop(job);
        }
        check_errors(
            env,
            &options,
            error_writer,
            "exiting with condition generation errors",
        )?;
    } else if !roots.is_empty() {
        info!(
            "running {} solver jobs with at most {} processes",
            root_count,
            options.backend.proc_cores.max(1)
        );
        let now = Instant::now();
        let mut first_error = None;
        BoogieWrapper::run_boogie_pipeline(
            &options.backend,
            root_count,
            |root_index| {
                let error_count = env.error_count();
                let path = verification_output_path(&output_base_file, root_count, root_index);
                let (job, duration) = generate_boogie_job(
                    env,
                    &options,
                    &targets,
                    package_mono_info.as_ref(),
                    path,
                    roots[root_index].clone(),
                    package_deadline,
                )?;
                gen_durations.push(duration);
                if env.error_count() > error_count {
                    // Preserve diagnostics from all roots before failing generation.
                    if !job.output_existed && !options.backend.keep_artifacts {
                        fs::remove_file(&job.path).unwrap_or_default();
                    }
                    for (remaining_index, root) in roots.iter().enumerate().skip(root_index + 1) {
                        if package_error_limit_reached(env, &options, initial_error_count) {
                            break;
                        }
                        let path = verification_output_path(
                            &output_base_file,
                            root_count,
                            remaining_index,
                        );
                        let (job, duration) = generate_boogie_job(
                            env,
                            &options,
                            &targets,
                            package_mono_info.as_ref(),
                            path,
                            root.clone(),
                            package_deadline,
                        )?;
                        gen_durations.push(duration);
                        if !job.output_existed && !options.backend.keep_artifacts {
                            fs::remove_file(job.path).unwrap_or_default();
                        }
                    }
                    check_errors(
                        env,
                        &options,
                        error_writer,
                        "exiting with condition generation errors",
                    )?;
                }
                Ok((
                    job.path.clone(),
                    job.process_timeout,
                    job.seed_handoff_after,
                    job.process_deadline,
                    job,
                ))
            },
            |root_index, job, seed, result, duration| {
                let mut job_options = options.backend.clone();
                job_options.proc_cores = 1;
                job_options.hard_timeout_secs = job.process_timeout;
                let boogie = BoogieWrapper {
                    env,
                    targets: &targets,
                    writer: &job.writer,
                    options: &job_options,
                };
                let status = match boogie.analyze_and_verify_output(&job.path, seed, result) {
                    Ok(status) => status,
                    Err(err) => {
                        if first_error
                            .as_ref()
                            .is_none_or(|(first_index, _)| root_index < *first_index)
                        {
                            first_error = Some((root_index, err));
                        }
                        BoogieRunStatus::Errors
                    },
                };
                let timing = make_verification_timing(env, &job.root, duration, status);
                report_verification_timing(&timing);
                timings.push(timing);
                if !job.output_existed && !options.backend.keep_artifacts {
                    fs::remove_file(job.path).unwrap_or_default();
                }
                if package_error_limit_reached(env, &options, initial_error_count) {
                    return Err(anyhow!(
                        "stopping after {} verifier errors",
                        options.backend.package_error_limit
                    ));
                }
                Ok::<_, anyhow::Error>(())
            },
        )?;
        verify_durations.push(now.elapsed());
        if let Some((_, err)) = first_error {
            return Err(err);
        }
    }
    // Report durations.
    let dur_list = |ds: &[Duration]| {
        if ds.len() <= 10 {
            ds.iter()
                .map(|d| format!("{:.2}s", d.as_secs_f64()))
                .join("/")
        } else {
            format!(
                "{:.2}s total for {} jobs",
                ds.iter().sum::<Duration>().as_secs_f64(),
                ds.len()
            )
        }
    };
    info!(
        "{:.2}s build, {:.2}s trafo, {} gen, {} solve pipeline, total {:.2}s",
        build_duration.as_secs_f64(),
        trafo_duration.as_secs_f64(),
        dur_list(&gen_durations),
        dur_list(&verify_durations),
        start_time.elapsed().as_secs_f64()
    );
    if fail_on_verification_errors {
        check_errors(
            env,
            &options,
            error_writer,
            "exiting with verification errors",
        )?;
    }
    Ok(timings)
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

fn package_error_limit_reached(
    env: &GlobalEnv,
    options: &Options,
    initial_error_count: usize,
) -> bool {
    options.backend.package_error_limit > 0
        && env.error_count().saturating_sub(initial_error_count)
            >= options.backend.package_error_limit
}

pub fn generate_boogie(
    env: &GlobalEnv,
    options: &Options,
    verification_root: Option<VerificationRoot>,
    targets: &FunctionTargetsHolder,
) -> anyhow::Result<CodeWriter> {
    let writer = CodeWriter::new(env.internal_loc());
    add_prelude(env, &options.backend, &writer)?;
    let mut translator =
        BoogieTranslator::new(env, &options.backend, verification_root, targets, &writer);
    translator.translate();
    Ok(writer)
}

pub fn verify_boogie(
    env: &GlobalEnv,
    options: &Options,
    targets: &FunctionTargetsHolder,
    writer: CodeWriter,
) -> anyhow::Result<()> {
    let output_existed = write_boogie(&options.output_path, &writer)?;
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

fn write_boogie(output_path: &str, writer: &CodeWriter) -> anyhow::Result<bool> {
    let output_existed = Path::new(output_path).exists();
    debug!("writing boogie to `{}`", output_path);
    writer.process_result(|result| fs::write(output_path, result))?;
    Ok(output_existed)
}

fn generate_boogie_job(
    env: &GlobalEnv,
    options: &Options,
    targets: &FunctionTargetsHolder,
    package_mono_info: &mono_analysis::MonoInfo,
    path: String,
    root: VerificationRoot,
    package_deadline: Option<Instant>,
) -> anyhow::Result<(BoogieJob, Duration)> {
    let remaining_package_time = package_deadline
        .map(|deadline| {
            deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| anyhow!("package verification deadline exceeded"))
        })
        .transpose()?;
    let root_timeout = BoogieTranslator::verification_timeout(env, &options.backend, &root);
    let mut process_timeout = options.backend.process_timeout_secs(root_timeout);
    if let Some(remaining) = remaining_package_time {
        let remaining_secs = remaining
            .as_secs()
            .saturating_add(u64::from(remaining.subsec_nanos() > 0))
            .max(1);
        process_timeout = if process_timeout == 0 {
            remaining_secs
        } else {
            process_timeout.min(remaining_secs)
        };
    }
    let seed_handoff_after = options.backend.seed_handoff_after(root_timeout);
    let mut root_options = options.clone();
    root_options.output_path = path.clone();
    let now = Instant::now();
    let root_mono_info = MonoAnalysisProcessor::analyze_for_root(env, targets, root.clone());
    env.set_extension(root_mono_info);
    let generated = generate_boogie(env, &root_options, Some(root.clone()), targets);
    env.set_extension(package_mono_info.clone());
    let writer = generated?;
    let duration = now.elapsed();
    if package_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
        return Err(anyhow!("package verification deadline exceeded"));
    }
    let output_existed = write_boogie(&path, &writer)?;
    Ok((
        BoogieJob {
            root,
            path,
            writer,
            output_existed,
            process_timeout,
            seed_handoff_after,
            process_deadline: package_deadline,
        },
        duration,
    ))
}

fn verification_output_path(
    output_base_file: &str,
    root_count: usize,
    root_index: usize,
) -> String {
    if root_count > 1 {
        Path::new(output_base_file)
            .with_extension(format!("vc_{:04}.bpl", root_index + 1))
            .to_string_lossy()
            .to_string()
    } else {
        output_base_file.to_string()
    }
}

fn verification_output_base(
    options: &Options,
) -> anyhow::Result<(String, Option<tempfile::TempDir>)> {
    if options.backend.keep_artifacts {
        return Ok((options.output_path.clone(), None));
    }
    let dir = tempfile::Builder::new().prefix("move-prover-").tempdir()?;
    let output = dir.path().join("output.bpl").to_string_lossy().to_string();
    Ok((output, Some(dir)))
}

fn make_verification_timing(
    env: &GlobalEnv,
    root: &VerificationRoot,
    duration: Duration,
    status: BoogieRunStatus,
) -> VerificationTiming {
    let fun = env.get_function(root.fun);
    let type_display = fun.get_type_display_ctx();
    let inst = if root.inst.is_empty() {
        String::new()
    } else {
        format!(
            "<{}>",
            root.inst
                .iter()
                .map(|ty| ty.display(&type_display).to_string())
                .join(", ")
        )
    };
    VerificationTiming {
        function: fun.get_full_name_str(),
        verification_condition: format!("{}{} [{}]", fun.get_full_name_str(), inst, root.variant),
        duration,
        status,
    }
}

fn report_verification_timing(timing: &VerificationTiming) {
    let result = if timing.status == BoogieRunStatus::Ok {
        "ok"
    } else {
        "failed"
    };
    info!(
        "{:>6.3}s {} {}",
        timing.duration.as_secs_f64(),
        result,
        timing.verification_condition
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_output_is_temporary_unless_kept() {
        let options = Options::default();
        let (output, temporary_dir) = verification_output_base(&options).unwrap();
        let temporary_dir = temporary_dir.unwrap();
        assert_eq!(Path::new(&output).parent(), Some(temporary_dir.path()));
        let temporary_path = temporary_dir.path().to_path_buf();
        drop(temporary_dir);
        assert!(!temporary_path.exists());

        let options = Options {
            output_path: "kept.bpl".to_string(),
            backend: move_prover_boogie_backend::options::BoogieOptions {
                keep_artifacts: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let (output, temporary_dir) = verification_output_base(&options).unwrap();
        assert_eq!(output, "kept.bpl");
        assert!(temporary_dir.is_none());
    }
}
