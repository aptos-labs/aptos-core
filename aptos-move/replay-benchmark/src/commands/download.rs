// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::{build_debugger, build_debugger_with_db, RestAPI},
    workload::TransactionBlock,
};
use anyhow::{anyhow, bail};
use aptos_types::transaction::{Transaction, Version};
use clap::Parser;
use std::path::PathBuf;
use tokio::fs;

#[derive(Parser)]
#[command(about = "Downloads transactions and saves them locally")]
pub struct DownloadCommand {
    #[clap(flatten)]
    rest_api: RestAPI,

    #[clap(long, help = "Path to the txn database")]
    db_path: Option<String>,

    #[clap(long, help = "Whether to use the txn database")]
    use_db: bool,

    #[clap(
        long,
        help = "Path to the file where the downloaded transactions will be saved"
    )]
    transactions_file: String,

    #[clap(long, help = "Version of the first transaction to benchmark")]
    begin_version: Version,

    #[clap(
        long,
        help = "End version of transaction range (exclusive) selected for benchmarking"
    )]
    end_version: Version,
}

impl DownloadCommand {
    /// Downloads a range of transactions, and saves them locally.
    pub async fn download_transactions(self) -> anyhow::Result<()> {
        if self.begin_version >= self.end_version {
            bail!(
                "Transaction versions should be a valid semi-open interval [b, e).\
                 Instead got begin: {}, end: {}",
                self.begin_version,
                self.end_version,
            );
        }

        let debugger = if self.use_db {
            build_debugger_with_db(self.db_path.unwrap())?
        } else {
            build_debugger(self.rest_api.rest_endpoint.unwrap(), self.rest_api.api_key)?
        };

        // Explicitly get transaction corresponding to the end, so we can verify that blocks are
        // fully selected.
        let limit = self.end_version - self.begin_version + 1;
        let (mut txns, _) = debugger
            .get_committed_transactions(self.begin_version, limit)
            .await?;

        if !txns[0].is_block_start() {
            bail!(
                "First transaction {} must be a block start, but it is not",
                self.begin_version
            );
        }
        if !txns.pop().unwrap().is_block_start() {
            bail!(
                "All transactions in the block must be selected, transaction {} is not a block \
                end",
                self.end_version - 1
            );
        }

        let txn_blocks = partition(self.begin_version, txns);
        println!(
            "Downloaded {} blocks with {} transactions in total: versions [{}, {})",
            txn_blocks.len(),
            limit,
            self.begin_version,
            self.end_version,
        );

        let bytes = bcs::to_bytes(&txn_blocks)
            .map_err(|err| anyhow!("Error when serializing blocks of transactions: {:?}", err))?;
        fs::write(PathBuf::from(&self.transactions_file), &bytes).await?;
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
    use aptos_crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature},
        HashValue, Uniform,
    };
    use aptos_types::{
        block_metadata::BlockMetadata,
        chain_id::ChainId,
        transaction::{EntryFunction, RawTransaction, SignedTransaction},
    };
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        DownloadCommand::command().debug_assert();
    }

    fn user_transaction() -> Transaction {
        // The actual values for signed transaction do not matter.
        let entry_func = EntryFunction::new(
            ModuleId::new(AccountAddress::ONE, Identifier::new("foo").unwrap()),
            Identifier::new("foo_func").unwrap(),
            vec![],
            vec![],
        );
        let raw_txn = RawTransaction::new_entry_function(
            AccountAddress::ONE,
            0,
            entry_func,
            100,
            1,
            10,
            ChainId::test(),
        );

        let mut rng = StdRng::from_seed([0; 32]);
        let pub_key = Ed25519PublicKey::from(&Ed25519PrivateKey::generate(&mut rng));
        let signature = Ed25519Signature::dummy_signature();
        let signed_txn = SignedTransaction::new(raw_txn, pub_key, signature);

        Transaction::UserTransaction(signed_txn)
    }

    fn block_metadata() -> Transaction {
        // The actual values for block metadata do not matter.
        let block_metadata = BlockMetadata::new(
            HashValue::zero(),
            0,
            0,
            AccountAddress::ONE,
            vec![],
            vec![],
            100,
        );
        Transaction::BlockMetadata(block_metadata)
    }

    #[test]
    fn test_block_partition_1() {
        let txns = vec![block_metadata(), block_metadata(), block_metadata()];
        let blocks = partition(1, txns);
        assert_eq!(blocks.len(), 3);

        assert_eq!(blocks[0].begin_version, 1);
        assert_eq!(blocks[0].transactions.len(), 1);

        assert_eq!(blocks[1].begin_version, 2);
        assert_eq!(blocks[1].transactions.len(), 1);

        assert_eq!(blocks[2].begin_version, 3);
        assert_eq!(blocks[2].transactions.len(), 1);
    }

    #[test]
    fn test_block_partition_2() {
        let txns = vec![
            user_transaction(),
            user_transaction(),
            user_transaction(),
            block_metadata(),
            user_transaction(),
            user_transaction(),
            user_transaction(),
        ];

        let blocks = partition(0, txns);
        assert_eq!(blocks.len(), 2);

        assert_eq!(blocks[0].begin_version, 0);
        assert_eq!(blocks[0].transactions.len(), 3);

        assert_eq!(blocks[1].begin_version, 3);
        assert_eq!(blocks[1].transactions.len(), 4);
    }

    #[test]
    fn test_block_partition_3() {
        let txns = vec![user_transaction(), user_transaction(), user_transaction()];
        let blocks = partition(10, txns);
        assert_eq!(blocks.len(), 1);

        assert_eq!(blocks[0].begin_version, 10);
        assert_eq!(blocks[0].transactions.len(), 3);
    }
}
