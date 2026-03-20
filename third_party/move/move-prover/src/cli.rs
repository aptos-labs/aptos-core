// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! Functionality related to the command line interface of the Move prover.

use crate::inference::InferenceOptions;
use anyhow::anyhow;
use clap::Parser;
use codespan_reporting::diagnostic::Severity;
use legacy_move_compiler::shared::NumericalAddress;
use log::LevelFilter;
use move_model::{metadata::LanguageVersion, model::VerificationScope};
use move_prover_boogie_backend::{
    options,
    options::{BoogieOptions, CustomNativeOptions},
};
use move_prover_bytecode_pipeline::options::{AutoTraceLevel, ProverOptions};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Parse a verbosity string into a LevelFilter.
fn parse_verbosity(s: &str) -> Result<LevelFilter, String> {
    match s {
        "error" => Ok(LevelFilter::Error),
        "warn" => Ok(LevelFilter::Warn),
        "info" => Ok(LevelFilter::Info),
        "debug" => Ok(LevelFilter::Debug),
        _ => Err(format!(
            "invalid verbosity '{}': expected error, warn, info, or debug",
            s
        )),
    }
}

/// Represents options provided to the tool. Most of those options are configured via a toml
/// source; some over the command line flags.
///
/// NOTE: any fields carrying structured data must appear at the end for making
/// toml printing work. When changing this config, use `mvp --print-config` to
/// verify this works.
#[derive(Debug, Clone, Deserialize, Serialize, Parser)]
#[command(
    name = "mvp",
    version = "0.1.0",
    about = "The Move Prover",
    author = "The Move Contributors"
)]
#[command(
    after_help = "More options available via `--config file` or `--config-str str`. \
    Use `--print-config` to see format and current values. \
    See `move-prover/src/cli.rs::Option` for documentation."
)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// Path to the boogie output which represents the verification problem.
    #[arg(
        short = 'o',
        long = "output",
        value_name = "BOOGIE_FILE",
        default_value = "output.bpl"
    )]
    pub output_path: String,
    /// Verbosity level for logging.
    #[arg(short = 'v', long = "verbose", value_parser = parse_verbosity, default_value = "info")]
    pub verbosity_level: LevelFilter,
    /// The paths to the Move sources.
    /// Each source path should refer to either (1) a Move file or (2) a directory containing Move
    /// files, all to be compiled (e.g., not the root directory of a package---which contains
    /// Move.toml---but a specific subdirectory such as `sources`, `scripts`, and/or `tests`,
    /// depending on compilation mode).
    #[arg(value_name = "PATH_TO_SOURCE_FILE", num_args = 1..)]
    pub move_sources: Vec<String>,
    /// The paths to any dependencies for the Move sources. Those will not be verified but
    /// can be used by `move_sources`.
    /// Each move_dep path should refer to either (1) a Move file or (2) a directory containing
    /// Move files, all to be compiled (e.g., not the root directory of a package---which contains
    /// Move.toml---but a specific subdirectory such as `sources`).
    #[arg(short = 'd', long = "dependency", action = clap::ArgAction::Append, value_name = "PATH_TO_DEPENDENCY")]
    pub move_deps: Vec<String>,
    /// The values assigned to named addresses in the Move code being verified.
    #[arg(short = 'a', long = "named-addresses", num_args = 0..)]
    pub move_named_address_values: Vec<String>,
    /// Whether to run experimental pipeline
    #[arg(short = 'e', long)]
    pub experimental_pipeline: bool,
    /// The language version to use
    #[arg(long, value_parser = clap::value_parser!(LanguageVersion))]
    pub language_version: Option<LanguageVersion>,

    // --- CLI-only fields (not stored in TOML config) ---
    /// Path to a configuration file. Values in this file will be overridden by command line flags.
    #[arg(short = 'c', long = "config", value_name = "TOML_FILE")]
    #[serde(skip)]
    pub config: Option<String>,
    /// Inlines configuration string in toml syntax. Can be repeated.
    /// Use as in `-C=prover.opt=value -C=backend.opt=value`.
    #[arg(short = 'C', long = "config-str", conflicts_with = "config", num_args = 0.., value_name = "TOML_STRING")]
    #[serde(skip)]
    pub config_str: Vec<String>,
    /// Prints the effective toml configuration, then exits.
    #[arg(long)]
    #[serde(skip)]
    pub print_config: bool,
    /// Configures the prover to use Aptos natives.
    #[arg(long)]
    #[serde(skip)]
    pub aptos: bool,
    /// Enables automatic tracing of expressions in prover errors.
    #[arg(short = 't', long)]
    #[serde(skip)]
    pub trace: bool,
    /// The minimal level on which diagnostics are reported.
    #[arg(short = 's', long, value_parser = ["bug", "error", "warn", "note"])]
    #[serde(skip)]
    pub severity: Option<String>,
    /// Default scope of verification (can be overridden by `pragma verify=true|false`).
    #[arg(long, value_parser = ["public", "all", "none"], value_name = "SCOPE")]
    #[serde(skip)]
    pub verify: Option<String>,
    /// Only generate verification condition for one function.
    /// This overrides verification scope and can be overridden by the pragma verify=false.
    #[arg(long, value_name = "FUNCTION_NAME")]
    #[serde(skip)]
    pub verify_only: Option<String>,
    /// Only generate verification condition for given function,
    /// and generate a z3 trace file for analysis. The file will be stored
    /// at FUNCTION_NAME.z3log.
    #[arg(long, value_name = "FUNCTION_NAME")]
    #[serde(skip)]
    pub z3_trace: Option<String>,
    /// Inference options (spec inference mode, output format, output directory).
    #[command(flatten)]
    #[serde(skip)]
    pub inference: InferenceOptions,

    /// BEGIN OF STRUCTURED OPTIONS. DO NOT ADD VALUE FIELDS AFTER THIS
    /// Options for the prover.
    #[command(flatten)]
    pub prover: ProverOptions,
    /// Options for the prover backend.
    #[command(flatten)]
    pub backend: BoogieOptions,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            output_path: "output.bpl".to_string(),
            verbosity_level: LevelFilter::Info,
            move_sources: vec![],
            move_deps: vec![],
            move_named_address_values: vec![],
            prover: ProverOptions::default(),
            backend: BoogieOptions::default(),
            experimental_pipeline: false,
            language_version: Some(LanguageVersion::default()),
            config: None,
            config_str: vec![],
            print_config: false,
            aptos: false,
            trace: false,
            severity: None,
            verify: None,
            verify_only: None,
            z3_trace: None,
            inference: InferenceOptions::default(),
        }
    }
}

pub static DEFAULT_OPTIONS: Lazy<Options> = Lazy::new(Options::default);

impl Options {
    /// Creates options from toml configuration source.
    pub fn create_from_toml(toml_source: &str) -> anyhow::Result<Options> {
        Ok(toml::from_str(toml_source)?)
    }

    /// Creates options from toml configuration file.
    pub fn create_from_toml_file(toml_file: &str) -> anyhow::Result<Options> {
        Self::create_from_toml(&std::fs::read_to_string(toml_file)?)
    }

    // Creates options from command line arguments. This parses the arguments and terminates
    // the program on errors, printing usage information. The first argument is expected to be
    // the program name.
    pub fn create_from_args(args: &[String]) -> anyhow::Result<Options> {
        // First parse to validate arguments and extract --config/--config-str.
        let cli = Options::try_parse_from(args)?;
        // Start from either a config file, inline TOML, or the default options.
        // Then overlay CLI arguments using update_from, which only overwrites
        // fields explicitly provided on the command line.
        let mut options = if let Some(ref config_file) = cli.config {
            if config_file.is_empty() {
                Options::default()
            } else {
                Options::create_from_toml_file(config_file)?
            }
        } else if !cli.config_str.is_empty() {
            Options::create_from_toml(&cli.config_str[0])?
        } else {
            Options::default()
        };
        options.update_from(args);
        options.post_process()?;
        Ok(options)
    }

    /// Apply special-case post-processing for CLI-only flags.
    fn post_process(&mut self) -> anyhow::Result<()> {
        if self.trace {
            self.prover.auto_trace_level = AutoTraceLevel::VerifiedFunction;
        }
        if let Some(ref s) = self.severity {
            self.prover.report_severity = match s.as_str() {
                "bug" => Severity::Bug,
                "error" => Severity::Error,
                "warn" => Severity::Warning,
                "note" => Severity::Note,
                _ => unreachable!("clap validates severity values"),
            };
        }
        if let Some(ref s) = self.verify {
            self.prover.verify_scope = match s.as_str() {
                "public" => VerificationScope::Public,
                "all" => VerificationScope::All,
                "none" => VerificationScope::None,
                _ => unreachable!("clap validates verify values"),
            };
        }
        if let Some(ref name) = self.verify_only {
            self.prover.verify_scope = VerificationScope::Only(name.clone());
        }
        if let Some(ref fun_name) = self.z3_trace {
            self.prover.verify_scope = VerificationScope::Only(fun_name.clone());
            let short_name = if let Some(i) = fun_name.find("::") {
                &fun_name[i + 2..]
            } else {
                fun_name
            };
            self.backend.z3_trace_file = Some(format!("{}.z3log", short_name));
        }
        if self.aptos {
            self.backend.custom_natives = Some(CustomNativeOptions {
                template_bytes: include_bytes!(
                    "../../../../aptos-move/framework/src/aptos-natives.bpl"
                )
                .to_vec(),
                module_instance_names: options::custom_native_options(),
            });
            self.move_named_address_values
                .push("Extensions=0x1".to_string());
        }
        self.backend.num_instances = std::cmp::max(self.backend.num_instances, 1);
        self.backend.derive_options();

        if self.print_config {
            println!("{}", toml::to_string(self).unwrap());
            return Err(anyhow!("exiting"));
        }
        Ok(())
    }

    /// Sets up logging based on provided options. This should be called as early as possible
    /// and before any use of info!, warn! etc.
    pub fn setup_logging(&self) {
        move_compiler_v2::logging::setup_logging(Some(self.verbosity_level))
    }

    pub fn setup_logging_for_test(&self) {
        move_compiler_v2::logging::setup_logging_for_testing(Some(self.verbosity_level))
    }

    /// Convenience function to enable debugging (like high verbosity) on this instance.
    pub fn enable_debug(&mut self) {
        self.verbosity_level = LevelFilter::Debug;
    }

    /// Convenience function to set verbosity level to only show errors and warnings.
    pub fn set_quiet(&mut self) {
        self.verbosity_level = LevelFilter::Warn
    }
}

pub fn named_addresses_for_options(
    named_address_values: &BTreeMap<String, NumericalAddress>,
) -> Vec<String> {
    named_address_values
        .iter()
        .map(|(name, addr)| format!("{}={}", name, addr))
        .collect()
}
