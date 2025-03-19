// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Functions for running benchmarks and storing the results as files, as well as reading
// benchmark data back into memory.

use anyhow::anyhow;
use clap::{
    Arg,
    ArgAction::{Append, Set},
    Command,
};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use itertools::Itertools;
use move_model::model::{FunctionEnv, GlobalEnv, ModuleEnv, VerificationScope};
use move_prover::{
    check_errors, cli::Options, create_and_process_bytecode, create_init_num_operation_state,
    generate_boogie, verify_boogie,
};
use move_prover_bytecode_pipeline::options::ProverOptions;
use std::{
    fmt::Debug,
    fs::File,
    io::{LineWriter, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

// ============================================================================================
// Command line interface for running a benchmark

struct Runner {
    options: Options,
    out: LineWriter<File>,
    error_writer: StandardStream,
    per_function: bool,
}

pub fn benchmark(args: &[String]) {
    let cmd_line_parser = Command::new("benchmark")
        .version("0.1.0")
        .about("Benchmark program for the Move Prover")
        .author("The Diem Core Contributors")
        .arg(
            Arg::new("config")
                .action(Append)
                .short('c')
                .long("config")
                .num_args(1)
                .value_name("CONFIG_PATH")
                .help(
                    "path to a prover toml configuration file. The benchmark output will be \
                          stored at `CONFIG_PATH.data. This can be repeated so different \
                          configurations are benchmarked against the same set of input modules.",
                ),
        )
        .arg(
            Arg::new("function")
                .short('f')
                .long("func")
                .num_args(0)
                .action(Set)
                .help("whether benchmarking should happen per function; default is per module"),
        )
        .arg(
            Arg::new("aptos-natives")
                .short('a')
                .long("aptos")
                .num_args(0)
                .action(Set)
                .help("whether the aptos-natives should be included."),
        )
        .arg(
            Arg::new("dependencies")
                .action(Append)
                .long("dependency")
                .short('d')
                .num_args(1)
                .value_name("PATH_TO_DEPENDENCY")
                .help(
                    "path to a Move file, or a directory which will be searched for \
                    Move files, containing dependencies which will not be verified",
                ),
        )
        .arg(
            Arg::new("sources")
                .value_name("PATH_TO_SOURCE_FILE")
                .num_args(1..)
                .help("the source files to verify"),
        );
    let matches = cmd_line_parser.get_matches_from(args);
    let get_vec = |s: &str| -> Vec<String> {
        let vs = matches.get_many::<String>(s);
        vs.map_or_else(Vec::new, |v| v.cloned().collect())
    };
    let sources = get_vec("sources");
    let deps = get_vec("dependencies");
    let configs: Vec<Option<String>> = if matches.contains_id("config") {
        get_vec("config").into_iter().map(Some).collect_vec()
    } else {
        vec![None]
    };
    let per_function = matches.contains_id("function");
    let use_aptos_natives = matches.contains_id("aptos-natives");

    for config_spec in configs {
        let (config, out) = if let Some(config_file) = &config_spec {
            let extension = if per_function { "fun_data" } else { "mod_data" };
            let out = PathBuf::from(config_file)
                .with_extension(extension)
                .to_string_lossy()
                .to_string();
            (config_spec, out)
        } else {
            (None, "benchmark.data".to_string())
        };
        if let Err(s) = run_benchmark(
            &out,
            config.as_ref(),
            &sources,
            &deps,
            per_function,
            use_aptos_natives,
        ) {
            println!("ERROR: execution failed: {}", s);
        } else {
            println!("results stored at `{}`", out);
        }
    }
}

fn run_benchmark(
    out: &str,
    config_file_opt: Option<&String>,
    modules: &[String],
    dep_dirs: &[String],
    per_function: bool,
    use_aptos_natives: bool,
) -> anyhow::Result<()> {
    let mut options = if let Some(config_file) = config_file_opt {
        Options::create_from_toml_file(config_file)?
    } else {
        Options::default()
    };
    options.move_sources.append(&mut modules.to_vec());
    options.move_deps.append(&mut dep_dirs.to_vec());
    options.skip_attribute_checks = true;

    if use_aptos_natives {
        options.backend.custom_natives =
            Some(move_prover_boogie_backend::options::CustomNativeOptions {
                template_bytes: include_bytes!(
                    "../../../../../aptos-move/framework/src/aptos-natives.bpl"
                )
                .to_vec(),
                module_instance_names: move_prover_boogie_backend::options::custom_native_options(),
            });
    }
    // Do not allow any benchmark to run longer than 60s. If this is exceeded it usually
    // indicates a bug in boogie or the solver, because we already propagate soft timeouts, but
    // they are ignored.
    options.backend.hard_timeout_secs = 60;
    options.backend.global_timeout_overwrite = false;
    options.backend.vc_timeout = 400;
    options.set_quiet();
    options.backend.proc_cores = 1;
    options.backend.derive_options();
    options.setup_logging();

    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    let env = move_prover::create_move_prover_v2_model(&mut error_writer, options.clone())?;
    check_errors(&env, &options, &mut error_writer, "unexpected build errors")?;

    let config_descr = if let Some(config) = config_file_opt {
        config.clone()
    } else {
        "default".to_string()
    };

    let mut out = LineWriter::new(File::create(out)?);

    writeln!(out, "# config: {}", config_descr)?;
    writeln!(out, "# time  : {}", chrono::Utc::now())?;

    let mut runner = Runner {
        options,
        out,
        error_writer,
        per_function,
    };
    println!(
        "Starting benchmarking with config `{}`.\n\
        Notice that execution is slow because we enforce single core execution.",
        config_descr
    );
    runner.bench(&env)
}

impl Runner {
    fn bench(&mut self, env: &GlobalEnv) -> anyhow::Result<()> {
        for module in env.get_modules() {
            if module.is_target() {
                if self.per_function {
                    for fun in module.get_functions() {
                        self.bench_function(fun)?;
                    }
                } else {
                    self.bench_module(module)?;
                }
            }
        }
        Ok(())
    }

    fn bench_function(&mut self, fun: FunctionEnv<'_>) -> anyhow::Result<()> {
        print!("benchmarking function {} ..", fun.get_full_name_str());
        std::io::stdout().flush()?;

        // Scope verification to the given function
        let env = fun.module_env.env;
        self.options.prover.verify_scope = VerificationScope::Only(fun.get_full_name_str());
        ProverOptions::set(env, self.options.prover.clone());
        create_init_num_operation_state(env);
        // Run benchmark
        let (duration, status) = self.bench_function_or_module(fun.module_env.env)?;

        // Write data record of benchmark result
        writeln!(
            self.out,
            "{:<50} {:>15} {:>15}",
            fun.get_full_name_str(),
            duration.as_millis(),
            status
        )?;

        println!(" {:.3}s {}.", duration.as_secs_f64(), status);
        Ok(())
    }

    fn bench_module(&mut self, module: ModuleEnv<'_>) -> anyhow::Result<()> {
        print!("benchmarking module {} ..", module.get_full_name_str());
        std::io::stdout().flush()?;

        // Scope verification to the given module
        self.options.prover.verify_scope =
            VerificationScope::OnlyModule(module.get_full_name_str());
        ProverOptions::set(module.env, self.options.prover.clone());
        create_init_num_operation_state(module.env);
        // Run benchmark
        let (duration, status) = self.bench_function_or_module(module.env)?;

        // Write data record of benchmark result
        writeln!(
            self.out,
            "{:<40} {:>12} {:>12}",
            module.get_full_name_str(),
            duration.as_millis(),
            status
        )?;

        println!("\x08\x08{:.3}s {}.", duration.as_secs_f64(), status);
        Ok(())
    }

    fn bench_function_or_module(&mut self, env: &GlobalEnv) -> anyhow::Result<(Duration, String)> {
        // Create and process bytecode.
        let targets = create_and_process_bytecode(&self.options, env);
        check_errors(
            env,
            &self.options,
            &mut self.error_writer,
            "unexpected transformation errors",
        )?;

        // Generate boogie code.
        let code_writer = generate_boogie(env, &self.options, None, &targets)?;
        check_errors(
            env,
            &self.options,
            &mut self.error_writer,
            "unexpected boogie generation errors",
        )?;

        // Verify boogie, measuring duration.
        let now = Instant::now();
        verify_boogie(env, &self.options, &targets, code_writer)?;

        // Determine result status.
        let status = if env.error_count() > 0 {
            if env.has_diag("timeout") {
                "timeout"
            } else {
                "errors"
            }
        } else {
            "ok"
        };
        env.clear_diag();
        Ok((now.elapsed(), status.to_string()))
    }
}

// ============================================================================================
// Reading and manipulating benchmark data

/// Represents a benchmark.
#[derive(Clone, Debug)]
pub struct Benchmark {
    /// The simple name of the configuration.
    pub config: String,
    /// The associated data.
    pub data: Vec<BenchmarkData>,
}

/// A data entry of a benchmark.
#[derive(Clone, Debug)]
pub struct BenchmarkData {
    pub name: String,
    pub duration: usize,
    pub status: String,
}

/// Read benchmark from data file.
pub fn read_benchmark(data_file: &str) -> anyhow::Result<Benchmark> {
    let config = PathBuf::from(data_file)
        .with_extension("") // remove extension
        .file_name() // use simple filename
        .ok_or_else(|| anyhow!("invalid data file name"))?
        .to_string_lossy()
        .to_string();
    let content = std::fs::read_to_string(data_file)?;
    let mut data = vec![];
    for line in content.lines() {
        if line.starts_with('#') {
            continue;
        }
        let parts = line.split_whitespace().collect_vec();
        if parts.len() != 3 {
            return Err(anyhow!("bad data entry"));
        }
        let name = parts[0].to_string();
        let duration = parts[1].parse::<usize>()?;
        let status = parts[2].to_string();
        data.push(BenchmarkData {
            name,
            duration,
            status,
        });
    }
    Ok(Benchmark { config, data })
}

impl Benchmark {
    /// Sort the benchmark data by longest duration.
    pub fn sort(&mut self) {
        self.data
            .sort_by(|d1, d2| d1.duration.cmp(&d2.duration).reverse());
    }

    /// Shrink benchmark to the first count samples.
    pub fn take(&mut self, count: usize) {
        self.data.truncate(count)
    }

    /// Sum the durations of the samples in the benchmark.
    pub fn sum(&self) -> u32 {
        self.data
            .iter()
            .filter_map(|d| {
                if d.status == "ok" || d.status == "error" {
                    Some(d.duration as u32)
                } else {
                    None
                }
            })
            .sum()
    }
}

/// Print statistics for the given set of benchmarks.
/// TODO: would be nice to have a histogram instead of textual output.
pub fn stats_benchmarks(benchmarks: &[&Benchmark]) -> String {
    let baseline = benchmarks[0].sum() as f32 / 1000.0;
    let mut res = String::new();
    let config_width = benchmarks.iter().map(|b| b.config.len()).max().unwrap();
    for benchmark in benchmarks {
        let sum = benchmark.sum() as f32 / 1000.0;
        let factor = sum / baseline;
        res = format!(
            "{}\n{:width$}: {:.3}s tot, {:.3} rel",
            res,
            benchmark.config,
            sum,
            factor,
            width = config_width,
        );
    }
    res
}
