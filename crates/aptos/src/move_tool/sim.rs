// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::types::{CliCommand, CliResult, CliTypedResult},
    move_tool::ReplayNetworkSelection,
};
use aptos_rest_client::Client;
use aptos_transaction_simulation_session::{BlockTimestamp, Session};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
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

/// View a resource
#[derive(Debug, Parser)]
pub struct ViewResource {
    /// Path to a stored session
    #[clap(long)]
    session: PathBuf,

    /// Account under which the resource is stored
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    account: AccountAddress,

    /// Resource to view
    #[clap(long)]
    resource: StructTag,
}

#[async_trait]
impl CliCommand<Option<serde_json::Value>> for ViewResource {
    fn command_name(&self) -> &'static str {
        "view-resource"
    }

    async fn execute(self) -> CliTypedResult<Option<serde_json::Value>> {
        let mut session = Session::load(&self.session)?;

        Ok(session.view_resource(self.account, &self.resource)?)
    }
}

/// View a resource group
#[derive(Debug, Parser)]
pub struct ViewResourceGroup {
    /// Path to a stored session
    #[clap(long)]
    session: PathBuf,

    /// Account under which the resource group is stored
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    account: AccountAddress,

    /// Resource group to view
    #[clap(long)]
    resource_group: StructTag,

    /// If specified, derives an object address from the source address and an object
    #[clap(long)]
    derived_object_address: Option<AccountAddress>,
}

#[async_trait]
impl CliCommand<Option<serde_json::Value>> for ViewResourceGroup {
    fn command_name(&self) -> &'static str {
        "view-resource-group"
    }

    async fn execute(self) -> CliTypedResult<Option<serde_json::Value>> {
        let mut session = Session::load(&self.session)?;
        Ok(session.view_resource_group(
            self.account,
            &self.resource_group,
            self.derived_object_address,
        )?)
    }
}

/// Execute a new block
///
/// This executes a real block metadata transaction through the VM. The block prologue
/// updates the on-chain timestamp and may trigger an epoch change if enough time has
/// passed since the last reconfiguration.
///
/// If neither --timestamp-usecs nor --offset-usecs is provided, the block advances by
/// 1 microsecond. These two options are mutually exclusive.
#[derive(Debug, Parser)]
pub struct NewBlock {
    /// Path to a stored session
    #[clap(long)]
    session: PathBuf,

    /// Absolute block timestamp in microseconds.
    #[clap(long, conflicts_with = "offset_usecs")]
    timestamp_usecs: Option<u64>,

    /// Advance the current timestamp by this many microseconds.
    #[clap(long, conflicts_with = "timestamp_usecs")]
    offset_usecs: Option<u64>,
}

#[async_trait]
impl CliCommand<serde_json::Value> for NewBlock {
    fn command_name(&self) -> &'static str {
        "new-block"
    }

    async fn execute(self) -> CliTypedResult<serde_json::Value> {
        let mut session = Session::load(&self.session)?;

        let timestamp = match (self.timestamp_usecs, self.offset_usecs) {
            (Some(ts), _) => BlockTimestamp::Absolute(ts),
            (_, Some(offset)) => BlockTimestamp::Offset(offset),
            (None, None) => BlockTimestamp::Default,
        };

        let result = session.new_block(timestamp)?;
        serde_json::to_value(result).map_err(|e| anyhow::anyhow!(e).into())
    }
}

/// Advance to the next epoch
///
/// This calculates the minimum timestamp needed to cross the epoch boundary and
/// executes a new block at that timestamp, triggering a reconfiguration.
#[derive(Debug, Parser)]
pub struct AdvanceEpoch {
    /// Path to a stored session
    #[clap(long)]
    session: PathBuf,
}

#[async_trait]
impl CliCommand<serde_json::Value> for AdvanceEpoch {
    fn command_name(&self) -> &'static str {
        "advance-epoch"
    }

    async fn execute(self) -> CliTypedResult<serde_json::Value> {
        let mut session = Session::load(&self.session)?;

        let result = session.advance_epoch()?;
        serde_json::to_value(result).map_err(|e| anyhow::anyhow!(e).into())
    }
}

/// BETA: Commands for interacting with a local simulation session
///
/// BETA: Subject to change
#[derive(Subcommand)]
pub enum Sim {
    Init(Init),
    Fund(Fund),
    ViewResource(ViewResource),
    ViewResourceGroup(ViewResourceGroup),
    NewBlock(NewBlock),
    AdvanceEpoch(AdvanceEpoch),
}

impl Sim {
    pub async fn execute(self) -> CliResult {
        match self {
            Sim::Init(init) => init.execute_serialized_success().await,
            Sim::Fund(fund) => fund.execute_serialized_success().await,
            Sim::ViewResource(view_resource) => view_resource.execute_serialized().await,
            Sim::ViewResourceGroup(view_resource_group) => {
                view_resource_group.execute_serialized().await
            },
            Sim::NewBlock(new_block) => new_block.execute_serialized().await,
            Sim::AdvanceEpoch(advance_epoch) => advance_epoch.execute_serialized().await,
        }
    }
}
