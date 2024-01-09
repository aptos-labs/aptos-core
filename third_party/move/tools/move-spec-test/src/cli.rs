use clap::*;
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
