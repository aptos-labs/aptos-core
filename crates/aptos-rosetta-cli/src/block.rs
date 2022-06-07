// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{format_output, NetworkArgs, UrlArgs};
use aptos_rosetta::types::{BlockRequest, BlockResponse, PartialBlockIdentifier};
use clap::{Parser, Subcommand};

/// Block APIs
///
/// [API Spec](https://www.rosetta-api.org/docs/BlockApi.html)
#[derive(Debug, Subcommand)]
pub enum BlockCommand {
    Get(GetBlockCommand),
}

impl BlockCommand {
    pub async fn execute(self) -> anyhow::Result<String> {
        match self {
            BlockCommand::Get(inner) => format_output(inner.execute().await),
        }
    }
}

/// Get a block by transaction hash or version
///
/// [API Spec](https://www.rosetta-api.org/docs/BlockApi.html#block)
#[derive(Debug, Parser)]
pub struct GetBlockCommand {
    #[clap(long)]
    version: Option<u64>,
    #[clap(long)]
    txn_hash: Option<String>,
    #[clap(flatten)]
    network_args: NetworkArgs,
    #[clap(flatten)]
    url_args: UrlArgs,
}

impl GetBlockCommand {
    pub async fn execute(self) -> anyhow::Result<BlockResponse> {
        let request = BlockRequest {
            network_identifier: self.network_args.network_identifier(),
            block_identifier: PartialBlockIdentifier {
                index: self.version,
                hash: self.txn_hash,
            },
        };
        self.url_args.client().block(&request).await
    }
}
