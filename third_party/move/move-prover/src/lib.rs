// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::cli::Options;
use anyhow::anyhow;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream, WriteColor};
use itertools::Itertools;
#[allow(unused_imports)]
use log::{debug, info, warn};
use move_abigen::Abigen;
use move_docgen::Docgen;
use move_errmapgen::ErrmapGen;
use move_model::{
    code_writer::CodeWriter, metadata::LATEST_STABLE_COMPILER_VERSION_VALUE, model::GlobalEnv,
};
use move_prover_boogie_backend::{
    add_prelude, boogie_wrapper::BoogieWrapper, bytecode_translator::BoogieTranslator,
};
use move_prover_bytecode_pipeline::{
    number_operation::GlobalNumberOperationState, pipeline_factory,
};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub mod cli;

// =================================================================================================
// Prover API

pub fn run_move_prover_errors_to_stderr(options: Options) -> anyhow::Result<()> {
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    run_move_prover_v2(&mut error_writer, options, vec![])
}

pub fn run_move_prover_v2<W: WriteColor>(
    error_writer: &mut W,
    options: Options,
    experiments: Vec<String>,
) -> anyhow::Result<()> {
    let now = Instant::now();
    let mut env = create_move_prover_v2_model(error_writer, options.clone(), experiments)?;
    run_move_prover_with_model_v2(&mut env, error_writer, options, now)
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
        warn_deprecated: false,
        warn_of_deprecation_use_in_aptos_libs: false,
        warn_unused: false,
        whole_program: false,
        compile_test_code: false,
        compile_verify_code: true,
        external_checks: vec![],
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
            if !fun_env.is_inline() {
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

    // Until this point, prover and docgen have same code. Here we part ways.
    if options.run_docgen {
        return run_docgen(env, &options, error_writer, start_time);
    }
    // Same for ABI generator.
    if options.run_abigen {
        return run_abigen(env, &options, start_time);
    }
    // Same for the error map generator
    if options.run_errmapgen {
        return {
            run_errmapgen(env, &options, start_time);
            Ok(())
        };
    }

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
    let has_shards = options.backend.shards > 1;
    let output_base_file = options.output_path.clone();
    for shard in 0..options.backend.shards {
        // If there are shards, modify the output name
        if has_shards {
            options.output_path = Path::new(&output_base_file)
                .with_extension(format!("shard_{}.bpl", shard + 1))
                .to_string_lossy()
                .to_string();
        }
        // Generate boogie code.
        let now = Instant::now();
        let code_writer = generate_boogie(
            env,
            &options,
            if has_shards { Some(shard) } else { None },
            &targets,
        )?;
        gen_durations.push(now.elapsed());
        check_errors(
            env,
            &options,
            error_writer,
            "exiting with condition generation errors",
        )?;

        // Verify boogie code.
        let now = Instant::now();
        verify_boogie(env, &options, &targets, code_writer)?;
        verify_durations.push(now.elapsed());
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

pub fn generate_boogie(
    env: &GlobalEnv,
    options: &Options,
    shard: Option<usize>,
    targets: &FunctionTargetsHolder,
) -> anyhow::Result<CodeWriter> {
    let writer = CodeWriter::new(env.internal_loc());
    add_prelude(env, &options.backend, &writer)?;
    let mut translator = BoogieTranslator::new(env, &options.backend, shard, targets, &writer);
    translator.translate();
    Ok(writer)
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
                let dump_file = output_dir.join(format!("{}.mv.disas", output_prefix));
                fs::write(dump_file, out).expect("dumping disassembled module");
            }
        }
        for func_env in module_env.get_functions() {
            targets.add_target(&func_env)
        }
    }

    // Create processing pipeline and run it.
    let pipeline = if options.experimental_pipeline {
        pipeline_factory::experimental_pipeline()
    } else {
        pipeline_factory::default_pipeline_with_options(&options.prover)
    };

    if options.prover.dump_bytecode {
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
            &|_| {},
            || true,
        )
    } else {
        pipeline.run(env, &mut targets);
    }

    targets
}

// Tools using the Move prover top-level driver
// ============================================

// TODO: make those tools independent. Need to first address the todo to
// move the model builder into the move-model crate.

fn run_docgen<W: WriteColor>(
    env: &GlobalEnv,
    options: &Options,
    error_writer: &mut W,
    now: Instant,
) -> anyhow::Result<()> {
    let generator = Docgen::new(env, &options.docgen);
    let checking_elapsed = now.elapsed();
    info!("generating documentation");
    for (file, content) in generator.gen() {
        let path = PathBuf::from(&file);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path.as_path(), content)?;
    }
    let generating_elapsed = now.elapsed();
    info!(
        "{:.3}s checking, {:.3}s generating",
        checking_elapsed.as_secs_f64(),
        (generating_elapsed - checking_elapsed).as_secs_f64()
    );
    if env.has_errors() {
        env.report_diag(error_writer, options.prover.report_severity);
        Err(anyhow!("exiting with documentation generation errors"))
    } else {
        Ok(())
    }
}

fn run_abigen(env: &GlobalEnv, options: &Options, now: Instant) -> anyhow::Result<()> {
    let mut generator = Abigen::new(env, &options.abigen);
    let checking_elapsed = now.elapsed();
    info!("generating ABI files");
    generator.gen();
    for (file, content) in generator.into_result() {
        let path = PathBuf::from(&file);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path.as_path(), content)?;
    }
    let generating_elapsed = now.elapsed();
    info!(
        "{:.3}s checking, {:.3}s generating",
        checking_elapsed.as_secs_f64(),
        (generating_elapsed - checking_elapsed).as_secs_f64()
    );
    Ok(())
}

fn run_errmapgen(env: &GlobalEnv, options: &Options, now: Instant) {
    let mut generator = ErrmapGen::new(env, &options.errmapgen);
    let checking_elapsed = now.elapsed();
    info!("generating error map");
    generator.gen();
    generator.save_result();
    let generating_elapsed = now.elapsed();
    info!(
        "{:.3}s checking, {:.3}s generating",
        checking_elapsed.as_secs_f64(),
        (generating_elapsed - checking_elapsed).as_secs_f64()
    );
}
