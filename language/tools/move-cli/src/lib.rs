// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_package::BuildConfig;

pub mod base;
pub mod experimental;
pub mod package;
pub mod sandbox;

/// Default directory where saved Move resources live
pub const DEFAULT_STORAGE_DIR: &str = "storage";

/// Default directory for build output
pub const DEFAULT_BUILD_DIR: &str = ".";

/// Extension for resource and event files, which are in BCS format
const BCS_EXTENSION: &str = "bcs";

use anyhow::Result;
use move_core_types::{
    account_address::AccountAddress, errmap::ErrorMapping, identifier::Identifier,
};
use move_vm_runtime::native_functions::NativeFunction;
use std::path::PathBuf;
use structopt::StructOpt;

type NativeFunctionRecord = (AccountAddress, Identifier, Identifier, NativeFunction);

#[derive(StructOpt)]
#[structopt(
    name = "move",
    about = "CLI frontend for Move compiler and VM",
    rename_all = "kebab-case"
)]
pub struct Move {
    /// Path to a package which the command should be run with respect to.
    #[structopt(
        long = "path",
        short = "p",
        global = true,
        parse(from_os_str),
        default_value = "."
    )]
    package_path: PathBuf,

    /// Print additional diagnostics if available.
    #[structopt(short = "v", global = true)]
    verbose: bool,

    /// Package build options
    #[structopt(flatten)]
    build_config: BuildConfig,
}

/// MoveCLI is the CLI that will be executed by the `move-cli` command
/// The `cmd` argument is added here rather than in `Move` to make it
/// easier for other crates to extend `move-cli`
#[derive(StructOpt)]
pub struct MoveCLI {
    #[structopt(flatten)]
    move_args: Move,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
pub enum Command {
    /// Execute a package command. Executed in the current directory or the closest containing Move
    /// package.
    #[structopt(name = "package")]
    Package {
        #[structopt(subcommand)]
        cmd: package::cli::PackageCommand,
    },
    /// Execute a sandbox command.
    #[structopt(name = "sandbox")]
    Sandbox {
        /// Directory storing Move resources, events, and module bytecodes produced by module publishing
        /// and script execution.
        #[structopt(long, default_value = DEFAULT_STORAGE_DIR, parse(from_os_str))]
        storage_dir: PathBuf,
        #[structopt(subcommand)]
        cmd: sandbox::cli::SandboxCommand,
    },
    /// (Experimental) Run static analyses on Move source or bytecode.
    #[structopt(name = "experimental")]
    Experimental {
        /// Directory storing Move resources, events, and module bytecodes produced by module publishing
        /// and script execution.
        #[structopt(long, default_value = DEFAULT_STORAGE_DIR, parse(from_os_str))]
        storage_dir: PathBuf,
        #[structopt(subcommand)]
        cmd: experimental::cli::ExperimentalCommand,
    },
}

pub fn run_cli(
    natives: Vec<NativeFunctionRecord>,
    error_descriptions: &ErrorMapping,
    move_args: &Move,
    cmd: &Command,
) -> Result<()> {
    match cmd {
        Command::Sandbox { storage_dir, cmd } => {
            cmd.handle_command(natives, error_descriptions, move_args, storage_dir)
        }
        Command::Experimental { storage_dir, cmd } => cmd.handle_command(move_args, storage_dir),
        Command::Package { cmd } => package::cli::handle_package_commands(
            &move_args.package_path,
            move_args.build_config.clone(),
            cmd,
            natives,
        ),
    }
}

pub fn move_cli(
    natives: Vec<NativeFunctionRecord>,
    error_descriptions: &ErrorMapping,
) -> Result<()> {
    let args = MoveCLI::from_args();
    run_cli(natives, error_descriptions, &args.move_args, &args.cmd)
}
