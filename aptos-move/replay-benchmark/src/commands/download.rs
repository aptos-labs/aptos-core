// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::{build_debugger, RestAPI},
    workload::TransactionBlock,
};
use anyhow::anyhow;
use aptos_types::transaction::{Transaction, Version};
use clap::Parser;
use std::path::PathBuf;
use tokio::fs;

#[derive(Parser)]
#[command(about = "Downloads transactions and saves them locally")]
pub struct DownloadCommand {
    #[clap(flatten)]
    rest_api: RestAPI,

    #[clap(long, help = "First transaction to include for benchmarking")]
    begin_version: Version,

    #[clap(long, help = "Last transaction to include for benchmarking")]
    end_version: Version,

    #[clap(
        long,
        help = "Path to the file where the downloaded transactions will be saved"
    )]
    output_file: String,
}

impl DownloadCommand {
    /// Downloads a range of transactions, and saves them locally.
    pub async fn download_and_save_transactions(self) -> anyhow::Result<()> {
        assert!(
            self.begin_version <= self.end_version,
            "Transaction versions should be a valid closed interval. Instead got begin: {}, end: {}",
            self.begin_version,
            self.end_version,
        );

        let debugger = build_debugger(self.rest_api.rest_endpoint, self.rest_api.api_key)?;
        let limit = self.end_version - self.begin_version + 1;
        let (txns, _) = debugger
            .get_committed_transactions(self.begin_version, limit)
            .await?;

        let txn_blocks = partition(self.begin_version, txns);
        let bytes = bcs::to_bytes(&txn_blocks)
            .map_err(|err| anyhow!("Error when serializing blocks of transactions: {:?}", err))?;
        fs::write(PathBuf::from(&self.output_file), &bytes).await?;
        Ok(())
    }
}

/// Partitions a sequence of transactions into blocks.
fn partition(begin_version: Version, txns: Vec<Transaction>) -> Vec<TransactionBlock> {
    let mut begin_versions_and_blocks = Vec::with_capacity(txns.len());

    let mut curr_begin = begin_version;
    let mut curr_block = Vec::with_capacity(txns.len());

    for txn in txns {
        if txn.is_block_start() && !curr_block.is_empty() {
            let block_size = curr_block.len();
            begin_versions_and_blocks.push(TransactionBlock {
                begin_version: curr_begin,
                transactions: std::mem::take(&mut curr_block),
            });
            curr_begin += block_size as Version;
        }
        curr_block.push(txn);
    }
    if !curr_block.is_empty() {
        begin_versions_and_blocks.push(TransactionBlock {
            begin_version: curr_begin,
            transactions: curr_block,
        });
    }

    begin_versions_and_blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        DownloadCommand::command().debug_assert();
    }
}
