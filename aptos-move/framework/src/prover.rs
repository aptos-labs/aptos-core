// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::build_model;
use codespan_reporting::{
    diagnostic::Severity,
    term::termcolor::{ColorChoice, StandardStream},
};
use log::{info, LevelFilter};
use move_core_types::account_address::AccountAddress;
use move_model::{
    metadata::{CompilerVersion, LanguageVersion},
    model::{GlobalEnv, VerificationScope},
};
use move_prover::cli::Options;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    time::Instant,
};
use tempfile::TempDir;

#[derive(Debug, Clone, clap::Parser, serde::Serialize, serde::Deserialize)]
pub struct ProverOptions {
    /// Verbosity level
    #[clap(long, short)]
    pub verbosity: Option<LevelFilter>,

    /// Filters targets out from the package. Any module with a matching file name will
    /// be a target, similar as with `cargo test`.
    #[clap(long, short)]
    pub filter: Option<String>,

    /// Scopes verification to the specified function. This can either be a name of the
    /// form "mod::func" or simply "func", in the later case every matching function is
    /// taken.
    #[clap(long, short)]
    pub only: Option<String>,

    /// Whether to display additional information in error reports. This may help
    /// debugging but also can make verification slower.
    #[clap(long, short)]
    pub trace: bool,

    /// Whether to use cvc5 as the smt solver backend. The environment variable
    /// `CVC5_EXE` should point to the binary.
    #[clap(long)]
    pub cvc5: bool,

    /// The depth until which stratified functions are expanded.
    #[clap(long, default_value_t = 6)]
    pub stratification_depth: usize,

    /// A seed for the prover.
    #[clap(long, default_value_t = 0)]
    pub random_seed: usize,

    /// The number of cores to use for parallel processing of verification conditions.
    #[clap(long, default_value_t = 4)]
    pub proc_cores: usize,

    /// A (soft) timeout for the solver, per verification condition, in seconds.
    #[clap(long, default_value_t = 40)]
    pub vc_timeout: usize,

    /// Whether to disable global timeout overwrite.
    /// With this flag set to true, the value set by "--vc-timeout" will be used globally
    #[clap(long, default_value_t = false)]
    pub disallow_global_timeout_to_be_overwritten: bool,

    /// Whether to check consistency of specs by injecting impossible assertions.
    #[clap(long)]
    pub check_inconsistency: bool,

    /// Whether to treat abort as inconsistency when checking consistency.
    /// Need to work together with check-inconsistency
    #[clap(long)]
    pub unconditional_abort_as_inconsistency: bool,

    /// Whether to keep loops as they are and pass them on to the underlying solver.
    #[clap(long)]
    pub keep_loops: bool,

    /// Number of iterations to unroll loops.
    #[clap(long)]
    pub loop_unroll: Option<u64>,

    /// Whether output for e.g. diagnosis shall be stable/redacted so it can be used in test
    /// output.
    #[clap(long)]
    pub stable_test_output: bool,

    /// Whether to dump intermediate step results to files.
    #[clap(long)]
    pub dump: bool,

    /// Whether to benchmark verification. If selected, each verification target in the
    /// current package will be verified independently with timing recorded. This attempts
    /// to detect timeouts. A benchmark report will be written to `prover_benchmark.fun_data` in the
    /// package directory. The command also writes a `prover_benchmark.svg` graphic, which
    /// is build from the data in the file above, comparing with any other `*.fun_data` files
    /// in the package directory. Thus, you can rename the data file to something like
    /// `prover_benchmark_v1.fun_data` and in the next run, compare benchmarks in the `.svg`
    /// file from multiple runs.
    #[clap(long = "benchmark")]
    pub benchmark: bool,

    /// Whether to skip verification of type instantiations of functions. This may miss
    /// some verification conditions if different type instantiations can create
    /// different behavior via type reflection or storage access, but can speed up
    /// verification.
    #[clap(long = "skip-instance-check")]
    pub skip_instance_check: bool,

    #[clap(skip)]
    pub for_test: bool,
}

impl Default for ProverOptions {
    fn default() -> Self {
        Self {
            verbosity: None,
            filter: None,
            only: None,
            trace: false,
            cvc5: false,
            stratification_depth: 6,
            random_seed: 0,
            proc_cores: 4,
            vc_timeout: 40,
            disallow_global_timeout_to_be_overwritten: false,
            check_inconsistency: false,
            unconditional_abort_as_inconsistency: false,
            keep_loops: false,
            loop_unroll: None,
            stable_test_output: false,
            dump: false,
            benchmark: false,
            for_test: false,
            skip_instance_check: false,
        }
    }
}

impl ProverOptions {
    /// Runs the move prover on the package.
    pub fn prove(
        self,
        dev_mode: bool,
        package_path: &Path,
        named_addresses: BTreeMap<String, AccountAddress>,
        bytecode_version: Option<u32>,
        compiler_version: Option<CompilerVersion>,
        language_version: Option<LanguageVersion>,
        skip_attribute_checks: bool,
        known_attributes: &BTreeSet<String>,
        experiments: &[String],
    ) -> anyhow::Result<()> {
        let now = Instant::now();
        let for_test = self.for_test;
        let benchmark = self.benchmark;
        let mut model = build_model(
            dev_mode,
            package_path,
            named_addresses,
            self.filter.clone(),
            bytecode_version,
            compiler_version,
            language_version,
            skip_attribute_checks,
            known_attributes.clone(),
            experiments.to_vec(),
        )?;
        let mut options = self.convert_options();
        options.language_version = language_version;
        options.model_builder.language_version = language_version.unwrap_or_default();
        if compiler_version.unwrap_or_default() >= CompilerVersion::V2_0
            || language_version
                .unwrap_or_default()
                .is_at_least(LanguageVersion::V2_0)
        {
            options.compiler_v2 = true;
        }
        // Need to ensure a distinct output.bpl file for concurrent execution. In non-test
        // mode, we actually want to use the static output.bpl for debugging purposes
        let _temp_holder = if for_test {
            let temp_dir = TempDir::new()?;
            std::fs::create_dir_all(temp_dir.path())?;
            options.output_path = temp_dir
                .path()
                .join("boogie.bpl")
                .to_string_lossy()
                .to_string();
            Some(temp_dir)
        } else {
            options.output_path = std::env::current_dir()?
                .join("boogie.bpl")
                .display()
                .to_string();
            None
        };
        options.backend.custom_natives =
            Some(move_prover_boogie_backend::options::CustomNativeOptions {
                template_bytes: include_bytes!("aptos-natives.bpl").to_vec(),
                module_instance_names: move_prover_boogie_backend::options::custom_native_options(),
            });
        if benchmark {
            // Special mode of benchmarking
            run_prover_benchmark(package_path, &mut model, options)?;
        } else {
            let mut writer = StandardStream::stderr(ColorChoice::Auto);
            if compiler_version.unwrap_or_default() == CompilerVersion::V1 {
                move_prover::run_move_prover_with_model(
                    &mut model,
                    &mut writer,
                    options,
                    Some(now),
                )?;
            } else {
                move_prover::run_move_prover_with_model_v2(&mut model, &mut writer, options, now)?;
            }
        }
        Ok(())
    }

    fn convert_options(self) -> move_prover::cli::Options {
        let verbosity_level = if let Some(level) = self.verbosity {
            level
        } else if self.for_test {
            LevelFilter::Warn
        } else {
            LevelFilter::Info
        };
        let opts = move_prover::cli::Options {
            output_path: "".to_string(),
            verbosity_level,
            prover: move_prover_bytecode_pipeline::options::ProverOptions {
                verify_scope: if let Some(name) = self.only {
                    VerificationScope::Only(name)
                } else {
                    VerificationScope::All
                },
                stable_test_output: self.stable_test_output,
                auto_trace_level: if self.trace {
                    move_prover_bytecode_pipeline::options::AutoTraceLevel::VerifiedFunction
                } else {
                    move_prover_bytecode_pipeline::options::AutoTraceLevel::Off
                },
                report_severity: Severity::Warning,
                dump_bytecode: self.dump,
                dump_cfg: false,
                check_inconsistency: self.check_inconsistency,
                unconditional_abort_as_inconsistency: self.unconditional_abort_as_inconsistency,
                skip_loop_analysis: self.keep_loops,
                ..Default::default()
            },
            backend: move_prover_boogie_backend::options::BoogieOptions {
                use_cvc5: self.cvc5,
                boogie_flags: vec![],
                generate_smt: self.dump,
                stratification_depth: self.stratification_depth,
                proc_cores: self.proc_cores,
                vc_timeout: self.vc_timeout,
                global_timeout_overwrite: !self.disallow_global_timeout_to_be_overwritten,
                keep_artifacts: self.dump,
                stable_test_output: self.stable_test_output,
                z3_trace_file: if self.dump {
                    Some("z3.trace".to_string())
                } else {
                    None
                },
                custom_natives: None,
                loop_unroll: self.loop_unroll,
                skip_instance_check: self.skip_instance_check,
                ..Default::default()
            },
            ..Default::default()
        };
        if self.for_test {
            opts.setup_logging_for_test();
        } else {
            opts.setup_logging()
        }
        opts
    }

    pub fn default_for_test() -> Self {
        Self {
            for_test: true,
            ..Self::default()
        }
    }
}

fn run_prover_benchmark(
    package_path: &Path,
    env: &mut GlobalEnv,
    mut options: Options,
) -> anyhow::Result<()> {
    info!("starting prover benchmark");
    // Determine sources and dependencies from the env
    let mut sources = BTreeSet::new();
    for module in env.get_modules() {
        let file_name = module.get_source_path().to_string_lossy().to_string();
        // If there is a `.spec.move` source, add this as well
        let spec_file_name = Path::new(&file_name).with_extension("spec.move");
        if spec_file_name.exists() {
            sources.insert(spec_file_name.to_string_lossy().to_string());
        }
        sources.insert(file_name);
    }

    // Enrich the prover options by the aliases in the env
    for (alias, address) in env.get_address_alias_map() {
        options.move_named_address_values.push(format!(
            "{}={}",
            alias.display(env.symbol_pool()),
            address.to_hex_literal()
        ))
    }

    // Create or override a prover_benchmark.toml in the package dir, reflection `options`
    let config_file = package_path.join("prover_benchmark.toml");
    let toml = toml::to_string(&options)?;
    std::fs::write(&config_file, toml)?;

    // Args for the benchmark API
    let mut args = vec![
        // Command name
        "bench".to_string(),
        // Benchmark by function not module
        "--func".to_string(),
        // Use as the config the file we derived from `options`
        "--config".to_string(),
        config_file.to_string_lossy().to_string(),
    ];

    // Add module filters
    for target in env.get_target_modules() {
        args.push("--module-filter".to_string());
        args.push(target.get_full_name_str())
    }

    args.extend(sources);
    move_prover_lab::benchmark::benchmark(&args);

    // The benchmark stores the result in `<config_file>.fun_data`, now plot it.
    // If there are any other `*.fun_data` files, add them to the plot.
    let mut args = vec![
        "plot".to_string(),
        format!(
            "--out={}",
            config_file
                .as_path()
                .with_extension("svg")
                .to_string_lossy()
        ),
        "--sort".to_string(),
    ];
    let main_data_file = config_file
        .as_path()
        .with_extension("fun_data")
        .to_string_lossy()
        .to_string();
    args.push(main_data_file.clone());
    let paths = fs::read_dir(package_path)?;
    for p in paths.flatten() {
        let p = p.path().as_path().to_string_lossy().to_string();
        // Only use this if its is not the main data file we already added
        if p.ends_with(".fun_data") && !p.ends_with("/prover_benchmark.fun_data") {
            args.push(p)
        }
    }
    move_prover_lab::plot::plot_svg(&args)
}
