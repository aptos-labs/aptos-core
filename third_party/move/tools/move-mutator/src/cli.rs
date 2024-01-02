use clap::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_OUTPUT_DIR: &str = "mutants_output";

/// Command line options for mutator
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
    /// The path where to put the output files.
    #[clap(long, short, value_parser, default_value = DEFAULT_OUTPUT_DIR)]
    pub out_mutant_dir: PathBuf,
    /// Indicates if mutants should be verified and made sure mutants can compile.
    #[clap(long, default_value = "true")]
    pub verify_mutants: bool,
    /// Indicates if the output files should be overwritten.
    #[clap(long, short)]
    pub no_overwrite: Option<bool>,
    /// Name of the filter to use for down sampling.
    #[clap(long)]
    pub downsample_filter: Option<String>,
    /// Optional configuration file. If provided, it will override the default configuration.
    #[clap(long, short, value_parser)]
    pub configuration_file: Option<PathBuf>,
}
