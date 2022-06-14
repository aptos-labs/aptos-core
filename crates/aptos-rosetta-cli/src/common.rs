// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{account, block, network};
use aptos_rosetta::{
    client::RosettaClient,
    types::{NetworkIdentifier, NetworkRequest},
};
use aptos_types::chain_id::ChainId;
use clap::Parser;
use serde::Serialize;

/// Aptos Rosetta CLI
///
/// Provides an implementation of [Rosetta](https://www.rosetta-api.org/docs/Reference.html) on Aptos.
#[derive(Debug, Parser)]
#[clap(name = "aptos-rosetta-cli", author, version, propagate_version = true)]
pub enum RosettaCliArgs {
    #[clap(subcommand)]
    Account(account::AccountCommand),
    #[clap(subcommand)]
    Block(block::BlockCommand),
    #[clap(subcommand)]
    Network(network::NetworkCommand),
}

impl RosettaCliArgs {
    pub async fn execute(self) -> anyhow::Result<String> {
        match self {
            RosettaCliArgs::Account(inner) => inner.execute().await,
            RosettaCliArgs::Block(inner) => inner.execute().await,
            RosettaCliArgs::Network(inner) => inner.execute().await,
        }
    }
}

/// Format output to a human readable form
pub fn format_output<T: Serialize>(input: anyhow::Result<T>) -> anyhow::Result<String> {
    input.map(|value| serde_json::to_string_pretty(&value).unwrap())
}

#[derive(Debug, Parser)]
pub struct UrlArgs {
    /// URL for the Aptos Rosetta API. e.g. http://localhost:8080
    #[clap(long, default_value = "http://localhost:8080")]
    rosetta_api_url: url::Url,
}

impl UrlArgs {
    pub fn client(self) -> RosettaClient {
        RosettaClient::new(self.rosetta_api_url)
    }
}

#[derive(Debug, Parser)]
pub struct NetworkArgs {
    /// ChainId to be used for the server e.g. TESTNET
    #[clap(long, default_value = "TESTING")]
    pub chain_id: ChainId,
}

impl NetworkArgs {
    pub fn network_identifier(self) -> NetworkIdentifier {
        self.chain_id.into()
    }

    pub fn network_request(self) -> NetworkRequest {
        NetworkRequest {
            network_identifier: self.network_identifier(),
        }
    }
}

/// Wrapper so that it's easy to tell that the output is an error
#[derive(Serialize)]
pub struct ErrorWrapper {
    pub error: String,
}
