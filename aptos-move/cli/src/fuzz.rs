// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_cli_common::{CliCommand, CliTypedResult};
use async_trait::async_trait;
use clap::Parser;
use move_fuzz::{
    cli::{run_on, FuzzCommand},
    language::LanguageSetting,
};
use std::path::PathBuf;

/// Fuzz a collection of Move packages
#[derive(Parser)]
pub struct Fuzz {
    /// Path to the project directory
    path: PathBuf,

    /// Subdirectories to be included in the analysis
    #[clap(long)]
    subdir: Vec<PathBuf>,

    /// Choose a language version
    #[clap(long, default_value = "2.3+")]
    language: LanguageSetting,

    /// Named alias declarations
    #[clap(long)]
    alias: Vec<String>,

    /// Resource account declaration
    #[clap(long)]
    resource: Vec<String>,

    /// Execute in-place instead of copying over the directory to a tempdir
    #[clap(long)]
    in_place: bool,

    /// Skip automated update of dependencies
    #[clap(long)]
    skip_deps_update: bool,

    /// Print additional diagnostics if available.
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Command
    #[clap(subcommand)]
    command: FuzzCommand,
}

#[async_trait]
impl CliCommand<&'static str> for Fuzz {
    fn command_name(&self) -> &'static str {
        "Fuzz"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let Self {
            path,
            subdir,
            language,
            alias,
            resource,
            in_place,
            skip_deps_update,
            verbose,
            command,
        } = self;
        run_on(
            path,
            subdir,
            language,
            alias,
            resource,
            in_place,
            skip_deps_update,
            verbose,
            command,
        )?;
        Ok("succeeded")
    }
}
