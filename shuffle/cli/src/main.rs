// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use std::{fs, path::PathBuf};
use structopt::StructOpt;
use url::Url;

mod account;
mod build;
mod console;
mod deploy;
mod new;
mod node;
mod shared;
mod test;
mod transactions;

#[tokio::main]
pub async fn main() -> Result<()> {
    let subcommand = Subcommand::from_args();
    match subcommand {
        Subcommand::New { blockchain, path } => new::handle(blockchain, path),
        Subcommand::Node {} => node::handle(),
        Subcommand::Build { project_path } => {
            build::handle(&normalized_project_path(project_path)?)
        }
        Subcommand::Deploy { project_path } => {
            deploy::handle(&normalized_project_path(project_path)?)
        }
        Subcommand::Account { root } => account::handle(root),
        Subcommand::Test { project_path } => test::handle(&normalized_project_path(project_path)?),
        Subcommand::Console {
            project_path,
            network,
            key_path,
            address,
        } => console::handle(
            &normalized_project_path(project_path)?,
            network,
            &normalized_key_path(key_path)?,
            &normalized_address(address)?,
        ),
        Subcommand::Transactions { network, tail } => {
            transactions::handle(normalized_network(network.as_str())?, should_tail(tail)).await
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "shuffle", about = "CLI frontend for Shuffle toolset")]
pub enum Subcommand {
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
    #[structopt(about = "Compiles the Move package and generates typescript files")]
    Build {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,
    },
    #[structopt(about = "Publishes the main move package using the account as publisher")]
    Deploy {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,
    },
    Account {
        #[structopt(short, long, help = "Creates account from mint.key passed in by user")]
        root: Option<PathBuf>,
    },
    #[structopt(about = "Starts a REPL for onchain inspection")]
    Console {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,

        #[structopt(short, long)]
        network: Option<String>,

        #[structopt(short, long, requires("address"))]
        key_path: Option<PathBuf>,

        #[structopt(short, long, requires("key-path"))]
        address: Option<String>,
    },
    #[structopt(about = "Runs end to end .ts tests")]
    Test {
        #[structopt(short, long)]
        project_path: Option<PathBuf>,
    },
    #[structopt(
        about = "Captures last 10 transactions and continuously polls for new transactions"
    )]
    Transactions {
        #[structopt(short, long)]
        network: String,

        #[structopt(short, help = "Blocks and streams future transactions as they happen")]
        tail: Option<Option<bool>>,
    },
}

fn normalized_project_path(project_path: Option<PathBuf>) -> Result<PathBuf> {
    match project_path {
        Some(path) => Ok(path),
        None => shared::get_shuffle_project_path(&std::env::current_dir()?),
    }
}

fn normalized_address(account_address: Option<String>) -> Result<String> {
    let home = shared::Home::new(shared::get_home_path().as_path())?;
    match account_address {
        Some(address) => {
            if &address[0..2] != "0x" {
                Ok("0x".to_owned() + &address)
            } else {
                Ok(address)
            }
        }
        None => {
            if !home.get_account_path().is_dir() {
                return Err(anyhow!(
                    "An account hasn't been created yet! Run shuffle account first"
                ));
            }
            let address = fs::read_to_string(home.get_latest_address_path())?;
            Ok("0x".to_owned() + &address)
        }
    }
}

fn normalized_key_path(diem_root_key_path: Option<PathBuf>) -> Result<PathBuf> {
    let home = shared::Home::new(shared::get_home_path().as_path())?;
    match diem_root_key_path {
        Some(key_path) => Ok(key_path),
        None => {
            if !home.get_account_path().is_dir() {
                return Err(anyhow!(
                    "An account hasn't been created yet! Run shuffle account first"
                ));
            }
            Ok(PathBuf::from(home.get_latest_key_path()))
        }
    }
}

fn normalized_network(network: &str) -> Result<Url> {
    match Url::parse(network) {
        Ok(_res) => Ok(Url::parse(network)?),
        Err(_e) => Ok(Url::parse(("http://".to_owned() + network).as_str())?),
    }
}

fn should_tail(tail_flag: Option<Option<bool>>) -> bool {
    match tail_flag {
        Some(Some(val)) => val,
        Some(_val) => true,
        None => false,
    }
}
