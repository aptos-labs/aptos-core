// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    shared::{get_home_path, Home},
    test::TestCommand,
};
use anyhow::{anyhow, Result};
use diem_types::account_address::AccountAddress;
use std::{fs, path::PathBuf};
use structopt::StructOpt;

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
    let home = Home::new(get_home_path().as_path())?;
    let subcommand = Subcommand::from_args();
    match subcommand {
        Subcommand::New { blockchain, path } => new::handle(blockchain, path),
        Subcommand::Node { genesis } => node::handle(&home, genesis),
        Subcommand::Build { project_path } => {
            build::handle(&shared::normalized_project_path(project_path)?)
        }
        Subcommand::Deploy {
            project_path,
            network,
        } => deploy::handle(
            &home,
            &shared::normalized_project_path(project_path)?,
            shared::normalized_network(&home, network)?,
        ),
        Subcommand::Account { root } => account::handle(&home, root),
        Subcommand::Test { cmd } => test::handle(&home, cmd),
        Subcommand::Console {
            project_path,
            network,
            key_path,
            address,
        } => console::handle(
            &shared::normalized_project_path(project_path)?,
            shared::normalized_network(&home, network)?,
            &normalized_key_path(&home, key_path)?,
            normalized_address(&home, address)?,
        ),
        Subcommand::Transactions {
            network,
            tail,
            address,
            raw,
        } => {
            transactions::handle(
                shared::normalized_network(&home, network)?,
                unwrap_nested_boolean_option(tail),
                normalized_address(&home, address)?,
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

        #[structopt(short, long)]
        network: Option<String>,
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

fn normalized_address(home: &Home, account_address: Option<String>) -> Result<AccountAddress> {
    let normalized_string = match account_address {
        Some(input_address) => {
            if &input_address[0..2] != "0x" {
                "0x".to_owned() + &input_address
            } else {
                input_address
            }
        }
        None => get_latest_address(home)?,
    };
    Ok(AccountAddress::from_hex_literal(
        normalized_string.as_str(),
    )?)
}

fn get_latest_address(home: &Home) -> Result<String> {
    home.check_account_path_exists()?;
    Ok(
        AccountAddress::from_hex(fs::read_to_string(home.get_latest_address_path())?)?
            .to_hex_literal(),
    )
}

fn normalized_key_path(home: &Home, diem_root_key_path: Option<PathBuf>) -> Result<PathBuf> {
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

fn unwrap_nested_boolean_option(option: Option<Option<bool>>) -> bool {
    match option {
        Some(Some(val)) => val,
        Some(_val) => true,
        None => false,
    }
}
