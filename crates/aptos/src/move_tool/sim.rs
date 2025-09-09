// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::types::{CliCommand, CliResult, CliTypedResult},
    move_tool::ReplayNetworkSelection,
};
use aptos_rest_client::Client;
use aptos_transaction_simulation_session::Session;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use move_core_types::account_address::AccountAddress;
use std::path::PathBuf;

/// Initializes a new simulation session
#[derive(Debug, Parser)]
pub struct Init {
    /// Path to the directory where the session data will be stored.
    #[clap(long)]
    path: PathBuf,

    /// If specified, starts the simulation by forking from a remote network state.
    #[clap(long)]
    network: Option<ReplayNetworkSelection>,

    /// The version of the network state to fork from.
    ///
    /// Only used if `--network` is specified.
    ///
    /// If not specified, the latest version of the network will be used.
    #[clap(long)]
    network_version: Option<u64>,

    /// API key for connecting to the fullnode.
    ///
    /// It is strongly recommended to specify an API key to avoid rate limiting.
    #[clap(long)]
    api_key: Option<String>,
}

#[async_trait]
impl CliCommand<()> for Init {
    fn command_name(&self) -> &'static str {
        "init"
    }

    async fn execute(self) -> CliTypedResult<()> {
        match self.network {
            Some(network) => {
                let network_version = match self.network_version {
                    Some(txn_id) => txn_id,
                    None => {
                        let client = Client::builder(network.to_base_url()?).build();
                        client.get_ledger_information().await?.inner().version
                    },
                };
                let base_url = network.to_base_url()?;
                let url = base_url.to_url();

                Session::init_with_remote_state(&self.path, url, network_version, self.api_key)?;
            },
            None => {
                Session::init(&self.path)?;
            },
        }

        Ok(())
    }
}

/// Funds an account with APT tokens
#[derive(Debug, Parser)]
pub struct Fund {
    /// Path to a stored session
    #[clap(long)]
    session: PathBuf,

    /// Account to fund, can be an address or a CLI profile name
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    account: AccountAddress,

    /// Funding amount, in Octa (10^-8 APT)
    #[clap(long)]
    amount: u64,
}

#[async_trait]
impl CliCommand<()> for Fund {
    fn command_name(&self) -> &'static str {
        "fund"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let mut session = Session::load(&self.session)?;

        session.fund_account(self.account, self.amount)?;

        Ok(())
    }
}

/// BETA: Commands for interacting with a local simulation session
///
/// BETA: Subject to change
#[derive(Subcommand)]
pub enum Sim {
    Init(Init),
    Fund(Fund),
}

impl Sim {
    pub async fn execute(self) -> CliResult {
        match self {
            Sim::Init(init) => init.execute_serialized_success().await,
            Sim::Fund(fund) => fund.execute_serialized_success().await,
        }
    }
}
