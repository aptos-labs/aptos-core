use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_OUTPUT_DIR: &str = "mutants_output";

/// Command line options for mutator
#[derive(Parser, Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct CLIOptions {
    /// The paths to the Move sources.
    #[clap(long, short, value_parser)]
    pub move_sources: Vec<PathBuf>,
    /// Module names to be mutated.
    #[clap(long)]
    pub mutate_modules: Option<Vec<String>>,
    /// The path where to put the output files.
    #[clap(long, short, value_parser)]
    pub out_mutant_dir: Option<PathBuf>,
    /// Indicates if mutants should be verified and made sure mutants can compile.
    #[clap(long, default_value = "false")]
    pub verify_mutants: bool,
    /// Indicates if the output files should be overwritten.
    #[clap(long, short)]
    pub no_overwrite: Option<bool>,
    /// Name of the filter to use for downsampling. Downsampling reduces the amount of mutants to the desired amount.
    #[clap(long, hide = true)]
    pub downsample_filter: Option<String>,
    /// Maximum number of mutants to be generated. If not specified, downsampling will be disabled. Currently only random filter is supported (mutants are removed randomly).
    #[clap(long)]
    pub downsample_num: Option<u64>,
    /// Optional configuration file. If provided, it will override the default configuration.
    #[clap(long, short, value_parser)]
    pub configuration_file: Option<PathBuf>,
}

impl Default for CLIOptions {
    // We need to implement default just because we need to specify the default value for out_mutant_dir.
    // Otherwise, out_mutant_dir would be empty. This is special case, when user won't specify any Options
    // (so the default value would be used), but define package_path (which is passed using other mechanism).
    fn default() -> Self {
        Self {
            move_sources: vec![],
            mutate_modules: None,
            out_mutant_dir: Some(PathBuf::from(DEFAULT_OUTPUT_DIR)),
            verify_mutants: true,
            no_overwrite: None,
            downsample_filter: None,
            downsample_num: None,
            configuration_file: None,
        }
    }
}
