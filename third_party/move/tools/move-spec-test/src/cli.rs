use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Command line options for specification test tool.
#[derive(Parser, Default, Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    /// The paths to the Move sources.
    #[clap(long, short, value_parser)]
    pub move_sources: Vec<PathBuf>,
    /// The paths to the Move sources to include.
    #[clap(long, short, value_parser)]
    pub include_only_files: Option<Vec<PathBuf>>,
    /// The paths to the Move sources to exclude.
    #[clap(long, short, value_parser)]
    pub exclude_files: Option<Vec<PathBuf>>,
    /// Optional configuration file for mutator tool.
    #[clap(long, value_parser)]
    pub mutator_conf: Option<PathBuf>,
    /// Optional configuration file for prover tool.
    #[clap(long, value_parser)]
    pub prover_conf: Option<PathBuf>,
    /// Extra arguments to pass to the prover.
    #[clap(long, value_parser)]
    pub extra_prover_args: Option<Vec<String>>,
}

/// This function creates a mutator CLI options from the given spec-test options.
#[must_use]
pub fn create_mutator_options(options: &Options) -> move_mutator::cli::Options {
    move_mutator::cli::Options {
        move_sources: options.move_sources.clone(),
        include_only_files: options.include_only_files.clone(),
        exclude_files: options.exclude_files.clone(),
        configuration_file: options.mutator_conf.clone(),
        ..Default::default()
    }
}

/// This function generates a prover CLI options from the given spec-test options.
///
/// # Errors
/// Errors are returned as `anyhow::Result`.
pub fn generate_prover_options(options: &Options) -> anyhow::Result<move_prover::cli::Options> {
    let prover_conf = if let Some(conf) = &options.prover_conf {
        move_prover::cli::Options::create_from_toml_file(conf.to_str().unwrap_or(""))?
    } else if let Some(args) = &options.extra_prover_args {
        move_prover::cli::Options::create_from_args(args)?
    } else {
        move_prover::cli::Options::default()
    };

    Ok(prover_conf)
}

/// This function checks if the mutator output path is provided in the configuration file.
/// We don't need to check if the mutator output path is provided in the options as they were created
/// from the spec-test options which does not allow setting it.
#[must_use]
pub fn check_mutator_output_path(options: &move_mutator::cli::Options) -> Option<PathBuf> {
    if let Some(conf) = &options.configuration_file {
        let c = move_mutator::configuration::Configuration::from_file(conf);
        if let Ok(c) = c {
            return c.project.out_mutant_dir;
        }
    };

    None
}
