// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use move_fuzz::{
    cli::{run_on, FuzzCommand},
    language::LanguageSetting,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "move-fuzz-dev")]
#[command(about = "Developer runner for move-fuzz without the full Aptos CLI shell")]
struct Args {
    /// Path to the project directory
    path: PathBuf,

    /// Subdirectories to be included in the analysis
    #[arg(long)]
    subdir: Vec<PathBuf>,

    /// Choose a language version
    #[arg(long, default_value = "2.3+")]
    language: LanguageSetting,

    /// Named alias declarations
    #[arg(long)]
    alias: Vec<String>,

    /// Resource account declaration
    #[arg(long)]
    resource: Vec<String>,

    /// Execute in-place instead of copying over the directory to a tempdir
    #[arg(long)]
    in_place: bool,

    /// Skip automated update of dependencies
    #[arg(long)]
    skip_deps_update: bool,

    /// Print additional diagnostics if available
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: FuzzCommand,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    run_on(
        args.path,
        args.subdir,
        args.language,
        args.alias,
        args.resource,
        args.in_place,
        args.skip_deps_update,
        args.verbose,
        args.command,
    )
}
