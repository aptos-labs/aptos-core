// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::test::TestCommand;
use anyhow::{anyhow, Result};
use diem_types::account_address::AccountAddress;
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
        Subcommand::Node { genesis } => node::handle(genesis),
        Subcommand::Build { project_path } => {
            build::handle(&shared::normalized_project_path(project_path)?)
        }
        Subcommand::Deploy { project_path } => {
            deploy::handle(&shared::normalized_project_path(project_path)?)
        }
        Subcommand::Account { root } => account::handle(root),
        Subcommand::Test { cmd } => test::handle(cmd),
        Subcommand::Console {
            project_path,
            network,
            key_path,
            address,
        } => console::handle(
            &shared::normalized_project_path(project_path)?,
            normalized_network(network)?,
            &normalized_key_path(key_path)?,
            normalized_address(address)?,
        ),
        Subcommand::Transactions {
            network,
            tail,
            address,
            raw,
        } => {
            transactions::handle(
                normalized_network(network)?,
                unwrap_nested_boolean_option(tail),
                normalized_address(address)?,
                unwrap_nested_boolean_option(raw),
            )
            .await
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
    Node {
        #[structopt(short, long, help = "Move package directory to be used for genesis")]
        genesis: Option<String>,
    },
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
        #[structopt(subcommand)]
        cmd: TestCommand,
    },
    #[structopt(
        about = "Captures last 10 transactions and continuously polls for new transactions from the account"
    )]
    Transactions {
        #[structopt(short, long)]
        network: Option<String>,

        #[structopt(
            short,
            long,
            help = "Writes out transactions without pretty formatting"
        )]
        raw: Option<Option<bool>>,

        #[structopt(
            short,
            long,
            help = "Captures and polls transactions deployed from a given address"
        )]
        address: Option<String>,

        #[structopt(short, help = "Blocks and streams future transactions as they happen")]
        tail: Option<Option<bool>>,
    },
}

fn normalized_address(account_address: Option<String>) -> Result<AccountAddress> {
    let normalized_string = match account_address {
        Some(input_address) => {
            if &input_address[0..2] != "0x" {
                "0x".to_owned() + &input_address
            } else {
                input_address
            }
        }
        None => get_latest_address()?,
    };
    Ok(AccountAddress::from_hex_literal(
        normalized_string.as_str(),
    )?)
}

fn get_latest_address() -> Result<String> {
    let home = shared::Home::new(shared::get_home_path().as_path())?;
    home.check_account_path_exists()?;
    Ok("0x".to_owned() + &fs::read_to_string(home.get_latest_address_path())?)
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

fn normalized_network(network: Option<String>) -> Result<Url> {
    let home = shared::Home::new(shared::get_home_path().as_path())?;
    match network {
        Some(input) => Ok(shared::build_url(
            input.as_str(),
            &home.read_networks_toml()?,
        )?),
        None => Ok(shared::build_url(
            shared::LOCALHOST_NETWORK_NAME,
            &home.read_networks_toml()?,
        )?),
    }
}

fn unwrap_nested_boolean_option(option: Option<Option<bool>>) -> bool {
    match option {
        Some(Some(val)) => val,
        Some(_val) => true,
        None => false,
    }
}
