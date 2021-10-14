// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

mod account;
mod console;
mod deploy;
mod new;
mod node;
mod shared;
mod test;

pub fn main() -> Result<()> {
    let subcommand = Subcommand::from_args();
    match subcommand {
        Subcommand::Account {} => account::handle(),
        Subcommand::New { blockchain, path } => new::handle(blockchain, path),
        Subcommand::Node {} => node::handle(),
        Subcommand::Deploy { project_path } => {
            deploy::handle(&normalized_project_path(project_path)?)
        }
        Subcommand::Console { project_path } => {
            console::handle(&normalized_project_path(project_path)?)
        }
        Subcommand::Test { project_path } => test::handle(&normalized_project_path(project_path)?),
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "shuffle", about = "CLI frontend for Shuffle toolset")]
pub enum Subcommand {
    #[structopt(about = "Creates new account with randomly generated private/public key")]
    Account {},
    #[structopt(about = "Creates a new shuffle project for Move development")]
    New {
        #[structopt(short, long, default_value = new::DEFAULT_BLOCKCHAIN)]
        blockchain: String,

        /// Path to destination dir
        #[structopt(parse(from_os_str))]
        path: PathBuf,
    },
    #[structopt(about = "Runs a local devnet with prefunded accounts")]
    Node {},
    #[structopt(about = "Publishes the main move package using the account as publisher")]
    Deploy {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,
    },
    #[structopt(about = "Starts a REPL for onchain inspection")]
    Console {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,
    },
    #[structopt(about = "Runs end to end .ts tests")]
    Test {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,
    },
}

fn normalized_project_path(project_path: Option<PathBuf>) -> Result<PathBuf> {
    match project_path {
        Some(path) => Ok(path),
        None => shared::get_shuffle_project_path(&std::env::current_dir()?),
    }
}
