// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::StructOpt;
use move_deps::{
    move_cli::{Command, Move},
    move_core_types::errmap::ErrorMapping,
    move_vm_types::gas_schedule::INITIAL_COST_SCHEDULE,
};

#[derive(StructOpt)]
pub struct AfCli {
    #[structopt(flatten)]
    move_args: Move,

    #[structopt(subcommand)]
    cmd: AfCommands,
}

#[derive(StructOpt)]
pub enum AfCommands {
    #[structopt(flatten)]
    Command(Command),
    // extra commands available only in af-cli can be added below
}

fn main() -> Result<()> {
    let error_descriptions: ErrorMapping = bcs::from_bytes(cached_framework_packages::error_map())?;
    let args = AfCli::parse();
    match &args.cmd {
        AfCommands::Command(cmd) => move_deps::move_cli::run_cli(
            aptos_vm::natives::aptos_natives(),
            &INITIAL_COST_SCHEDULE,
            &error_descriptions,
            &args.move_args,
            cmd,
        ),
    }
}
