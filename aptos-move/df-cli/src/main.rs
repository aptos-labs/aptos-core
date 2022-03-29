// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;
use move_cli::{Command, Move};
use move_core_types::errmap::ErrorMapping;
use move_vm_types::gas_schedule::INITIAL_COST_SCHEDULE;

#[derive(Parser)]
pub struct DfCli {
    #[clap(flatten)]
    move_args: Move,

    #[clap(subcommand)]
    cmd: DfCommands,
}

#[derive(Parser)]
pub enum DfCommands {
    #[clap(flatten)]
    Command(Command),
    // extra commands available only in df-cli can be added below
}

fn main() -> Result<()> {
    let error_descriptions: ErrorMapping =
        bcs::from_bytes(diem_framework_releases::current_error_descriptions())?;
    let args = DfCli::parse();
    match &args.cmd {
        DfCommands::Command(cmd) => move_cli::run_cli(
            aptos_vm::natives::aptos_natives(),
            &INITIAL_COST_SCHEDULE,
            &error_descriptions,
            &args.move_args,
            cmd,
        ),
    }
}
