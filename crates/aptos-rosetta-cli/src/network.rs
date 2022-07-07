// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{format_output, NetworkArgs, UrlArgs};
use aptos_rosetta::types::{NetworkListResponse, NetworkOptionsResponse, NetworkStatusResponse};
use clap::{Parser, Subcommand};

/// Network APIs
///
/// Used to get status of the current network and what is supported on the API
///
/// [API Spec](https://www.rosetta-api.org/docs/NetworkApi.html)
#[derive(Debug, Subcommand)]
pub enum NetworkCommand {
    List(NetworkListCommand),
    Options(NetworkOptionsCommand),
    Status(NetworkStatusCommand),
}

impl NetworkCommand {
    pub async fn execute(self) -> anyhow::Result<String> {
        match self {
            NetworkCommand::List(inner) => format_output(inner.execute().await),
            NetworkCommand::Options(inner) => format_output(inner.execute().await),
            NetworkCommand::Status(inner) => format_output(inner.execute().await),
        }
    }
}

/// Get list of available networks
///
/// [API Spec](https://www.rosetta-api.org/docs/NetworkApi.html#networklist)
#[derive(Debug, Parser)]
pub struct NetworkListCommand {
    #[clap(flatten)]
    url_args: UrlArgs,
}

impl NetworkListCommand {
    pub async fn execute(self) -> anyhow::Result<NetworkListResponse> {
        self.url_args.client().network_list().await
    }
}

/// Get network options
///
/// [API Spec](https://www.rosetta-api.org/docs/NetworkApi.html#networkoptions)
#[derive(Debug, Parser)]
pub struct NetworkOptionsCommand {
    #[clap(flatten)]
    network_args: NetworkArgs,
    #[clap(flatten)]
    url_args: UrlArgs,
}

impl NetworkOptionsCommand {
    pub async fn execute(self) -> anyhow::Result<NetworkOptionsResponse> {
        let request = self.network_args.network_request();
        self.url_args.client().network_options(&request).await
    }
}

/// Get network status
///
/// [API Spec](https://www.rosetta-api.org/docs/NetworkApi.html#networkstatus)
#[derive(Debug, Parser)]
pub struct NetworkStatusCommand {
    #[clap(flatten)]
    network_args: NetworkArgs,
    #[clap(flatten)]
    url_args: UrlArgs,
}

impl NetworkStatusCommand {
    pub async fn execute(self) -> anyhow::Result<NetworkStatusResponse> {
        let request = self.network_args.network_request();
        self.url_args.client().network_status(&request).await
    }
}
