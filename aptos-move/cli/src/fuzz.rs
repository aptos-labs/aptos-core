// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The `aptos move fuzz` subcommand.
//!
//! Thin `CliCommand` wrapper that forwards the project path and fuzzing options
//! to `move_fuzz::cli::run_on`.

use crate::move_types::OptimizationLevel;
use aptos_cli_common::{dir_default_to_current, CliCommand, CliTypedResult};
use async_trait::async_trait;
use clap::Parser;
use move_fuzz::{
    cli::{run_on, FuzzCommand},
    language::{LanguageSetting, OptLevel},
};
use move_model::metadata::{
    LanguageVersion, LATEST_STABLE_LANGUAGE_VERSION, LATEST_STABLE_LANGUAGE_VERSION_VALUE,
};
use std::path::PathBuf;

// NOTE: this command deliberately does not `#[clap(flatten)]` `MovePackageOptions`
// (`LintPackage` re-declares its options for the same reason). `fuzz` walks a directory
// tree, resolves every `Move.toml` under it, and must own the private key behind every
// named address so it can publish and sign inside the simulated network. Flattening would
// collide on `--language` and would advertise `--named-addresses`, `--output-dir`,
// `--override-std`, `--bytecode-version`, `--compiler-version`, `--fail-on-warning` and
// `--skip-checks-on-test-code` in `--help` while silently ignoring them. The options that
// do carry over reuse the exact names and value syntax of `MovePackageOptions`; only the
// defaults differ, and each says so.
/// Fuzz a collection of Move packages
#[derive(Parser)]
pub struct Fuzz {
    /// Path to a Move project, i.e., a directory holding one or more Move packages.
    /// Defaults to the current directory.
    #[clap(long, value_parser)]
    package_dir: Option<PathBuf>,

    /// Subdirectories to be included in the analysis
    #[clap(long)]
    subdir: Vec<PathBuf>,

    /// ...or --language LANGUAGE_VERSION
    /// Specify the language version to be supported.
    /// Defaults to the latest stable language version, as in the other
    /// `aptos move` subcommands.
    #[clap(long, value_parser = clap::value_parser!(LanguageVersion),
           alias = "language",
           default_value = LATEST_STABLE_LANGUAGE_VERSION,
           verbatim_doc_comment)]
    language_version: Option<LanguageVersion>,

    /// Select optimization level.  Choices are "none", "default", or "extra".
    /// Level "extra" may spend more time on expensive optimizations.
    /// Level "none" does no optimizations, possibly leading to use of too many runtime resources.
    /// Defaults to "extra" for fuzzing, so that the optimizer pipeline is exercised
    /// (the other `aptos move` subcommands default to "default").
    #[clap(long, alias = "optimization_level", value_parser = clap::value_parser!(OptimizationLevel))]
    optimize: Option<OptimizationLevel>,

    /// Named alias declarations
    #[clap(long)]
    alias: Vec<String>,

    /// Resource account declaration
    #[clap(long)]
    resource: Vec<String>,

    /// Execute in-place instead of copying over the directory to a tempdir
    #[clap(long)]
    in_place: bool,

    /// Skip pulling the latest git dependencies
    ///
    /// If you don't have a network connection, the compiler may fail due
    /// to no ability to pull git dependencies.  This will allow overriding
    /// this for local development.
    #[clap(long, alias = "skip-deps-update")]
    skip_fetch_latest_git_deps: bool,

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
            package_dir,
            subdir,
            language_version,
            optimize,
            alias,
            resource,
            in_place,
            skip_fetch_latest_git_deps,
            verbose,
            command,
        } = self;
        let language = LanguageSetting {
            version: language_version.unwrap_or(LATEST_STABLE_LANGUAGE_VERSION_VALUE),
            // NOTE: unlike the other `aptos move` subcommands, an unspecified level means
            // `extra` here: fuzzing wants the most aggressive optimizer pipeline.
            optimization: match optimize {
                None | Some(OptimizationLevel::Extra) => OptLevel::Extra,
                Some(OptimizationLevel::Default) => OptLevel::Default,
                Some(OptimizationLevel::None) => OptLevel::None,
            },
        };
        run_on(
            dir_default_to_current(package_dir)?,
            subdir,
            language,
            alias,
            resource,
            in_place,
            skip_fetch_latest_git_deps,
            verbose,
            command,
        )?;
        Ok("succeeded")
    }
}
