// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::StructOpt;
use move_deps::{
    move_cli::{Command, Move},
    move_core_types::errmap::ErrorMapping,
    move_vm_types::gas_schedule::INITIAL_COST_SCHEDULE,
};

use crossbeam_channel::unbounded;

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
    let (s1, r1) = unbounded();
    let (s2, r2) = (s1.clone(), r1.clone());
    let (s3, r3) = (s2.clone(), r2.clone());

    s1.send(10).unwrap();
    s2.send(20).unwrap();
    s3.send(30).unwrap();

    println!("{:?}", r3.recv());
    println!("{:?}", r1.recv());
    println!("{:?}", r2.recv());
    Ok(())
}
