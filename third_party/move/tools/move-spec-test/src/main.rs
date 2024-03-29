// Copyright © Eiger
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use clap::Parser;
use move_package::BuildConfig;
use move_spec_test::cli::CLIOptions;
use move_spec_test::run_spec_test;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Parser, Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Opts {
    /// The path where to put the output files.
    #[clap(long, short, value_parser)]
    pub package_path: Option<PathBuf>,
    /// Command line options for specification tester
    #[clap(flatten)]
    pub cli_options: CLIOptions,
    /// The build configuration options.
    #[clap(flatten)]
    pub build_config: BuildConfig,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::parse();
    let package_path = opts.package_path.unwrap_or(PathBuf::from("."));

    run_spec_test(&opts.cli_options, &opts.build_config, &package_path)
}
