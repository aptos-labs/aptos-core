// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{format_output, BlockArgs, NetworkArgs, UrlArgs};
use aptos_rosetta::types::{BlockRequest, BlockRequestMetadata, BlockResponse};
use clap::{Parser, Subcommand};

/// Block APIs
///
/// Used for pulling blocks from the blockchain
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
    #[clap(flatten)]
    block_args: BlockArgs,
    #[clap(flatten)]
    network_args: NetworkArgs,
    #[clap(flatten)]
    url_args: UrlArgs,
}

impl GetBlockCommand {
    pub async fn execute(self) -> anyhow::Result<BlockResponse> {
        let metadata = self
            .block_args
            .keep_all_transactions
            .map(|inner| BlockRequestMetadata {
                keep_empty_transactions: Some(inner),
            });
        let request = BlockRequest {
            network_identifier: self.network_args.network_identifier(),
            block_identifier: self.block_args.into(),
            metadata,
        };
        self.url_args.client().block(&request).await
    }
}
