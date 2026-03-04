// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    commands::{build_debugger, RestAPI},
    workload::TransactionBlock,
};
use anyhow::{anyhow, bail};
use aptos_types::transaction::{PersistedAuxiliaryInfo, Transaction, Version};
use clap::Parser;
use std::path::PathBuf;
use tokio::fs;

#[derive(Parser)]
#[command(about = "Downloads transactions and saves them locally")]
pub struct DownloadCommand {
    #[clap(flatten)]
    rest_api: RestAPI,

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

        let debugger = build_debugger(self.rest_api.rest_endpoint, self.rest_api.api_key)?;

        // Explicitly get transaction corresponding to the end, so we can verify that blocks are
        // fully selected.
        let limit = self.end_version - self.begin_version + 1;
        let (mut txns, _, mut auxiliary_infos) = debugger
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
        // Remove auxiliary info for the popped end transaction.
        auxiliary_infos.pop();

        let txn_blocks = partition(self.begin_version, txns, auxiliary_infos);
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

/// Partitions a sequence of transactions (and their auxiliary infos) into blocks.
fn partition(
    begin_version: Version,
    txns: Vec<Transaction>,
    auxiliary_infos: Vec<PersistedAuxiliaryInfo>,
) -> Vec<TransactionBlock> {
    assert_eq!(txns.len(), auxiliary_infos.len());

    let mut begin_versions_and_blocks = Vec::with_capacity(txns.len());

    let mut curr_begin = begin_version;
    let mut curr_block = Vec::with_capacity(txns.len());
    let mut curr_aux_infos = Vec::with_capacity(txns.len());

    for (txn, aux_info) in txns.into_iter().zip(auxiliary_infos) {
        if txn.is_block_start() && !curr_block.is_empty() {
            let block_size = curr_block.len();
            begin_versions_and_blocks.push(TransactionBlock {
                begin_version: curr_begin,
                transactions: std::mem::take(&mut curr_block),
                persisted_auxiliary_infos: std::mem::take(&mut curr_aux_infos),
            });
            curr_begin += block_size as Version;
        }
        curr_block.push(txn);
        curr_aux_infos.push(aux_info);
    }
    if !curr_block.is_empty() {
        begin_versions_and_blocks.push(TransactionBlock {
            begin_version: curr_begin,
            transactions: curr_block,
            persisted_auxiliary_infos: curr_aux_infos,
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

    fn aux_infos(n: usize) -> Vec<PersistedAuxiliaryInfo> {
        (0..n)
            .map(|i| PersistedAuxiliaryInfo::V1 {
                transaction_index: i as u32,
            })
            .collect()
    }

    #[test]
    fn test_block_partition_1() {
        let txns = vec![block_metadata(), block_metadata(), block_metadata()];
        let infos = aux_infos(txns.len());
        let blocks = partition(1, txns, infos);
        assert_eq!(blocks.len(), 3);

        assert_eq!(blocks[0].begin_version, 1);
        assert_eq!(blocks[0].transactions.len(), 1);
        assert_eq!(blocks[0].persisted_auxiliary_infos.len(), 1);

        assert_eq!(blocks[1].begin_version, 2);
        assert_eq!(blocks[1].transactions.len(), 1);
        assert_eq!(blocks[1].persisted_auxiliary_infos.len(), 1);

        assert_eq!(blocks[2].begin_version, 3);
        assert_eq!(blocks[2].transactions.len(), 1);
        assert_eq!(blocks[2].persisted_auxiliary_infos.len(), 1);
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
        let infos = aux_infos(txns.len());

        let blocks = partition(0, txns, infos);
        assert_eq!(blocks.len(), 2);

        assert_eq!(blocks[0].begin_version, 0);
        assert_eq!(blocks[0].transactions.len(), 3);
        assert_eq!(blocks[0].persisted_auxiliary_infos.len(), 3);

        assert_eq!(blocks[1].begin_version, 3);
        assert_eq!(blocks[1].transactions.len(), 4);
        assert_eq!(blocks[1].persisted_auxiliary_infos.len(), 4);
    }

    #[test]
    fn test_block_partition_3() {
        let txns = vec![user_transaction(), user_transaction(), user_transaction()];
        let infos = aux_infos(txns.len());
        let blocks = partition(10, txns, infos);
        assert_eq!(blocks.len(), 1);

        assert_eq!(blocks[0].begin_version, 10);
        assert_eq!(blocks[0].transactions.len(), 3);
        assert_eq!(blocks[0].persisted_auxiliary_infos.len(), 3);
    }
}
