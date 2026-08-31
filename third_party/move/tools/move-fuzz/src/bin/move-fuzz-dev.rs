// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use clap::Parser;
use move_fuzz::{
    cli::{run_on, FuzzCommand},
    language::{LanguageSetting, OptLevel},
};
use move_model::metadata::LanguageVersion;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "move-fuzz-dev")]
#[command(about = "Developer runner for move-fuzz without the full Aptos CLI shell")]
struct Args {
    /// Path to a Move project, i.e., a directory holding one or more Move packages.
    /// Defaults to the current directory.
    #[arg(long, value_parser)]
    package_dir: Option<PathBuf>,

    /// Subdirectories to be included in the analysis
    #[arg(long)]
    subdir: Vec<PathBuf>,

    /// Specify the language version to be supported. Defaults to 2.3.
    #[arg(long, alias = "language", default_value = "2.3")]
    language_version: LanguageVersion,

    /// Select optimization level.  Choices are "none", "default", or "extra".
    /// Defaults to "extra" for fuzzing.
    #[arg(long, default_value = "extra")]
    optimize: OptLevel,

    /// Named alias declarations
    #[arg(long)]
    alias: Vec<String>,

    /// Resource account declaration
    #[arg(long)]
    resource: Vec<String>,

    /// Execute in-place instead of copying over the directory to a tempdir
    #[arg(long)]
    in_place: bool,

    /// Skip pulling the latest git dependencies
    #[arg(long, alias = "skip-deps-update")]
    skip_fetch_latest_git_deps: bool,

    /// Print additional diagnostics if available
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: FuzzCommand,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let path = match args.package_dir {
        Some(path) => path,
        None => std::env::current_dir()?,
    };
    let language = LanguageSetting {
        version: args.language_version,
        optimization: args.optimize,
    };
    run_on(
        path,
        args.subdir,
        language,
        args.alias,
        args.resource,
        args.in_place,
        args.skip_fetch_latest_git_deps,
        args.verbose,
        args.command,
    )
}
