// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::{build_debugger, RestAPI},
    workload::{
        dump_and_check_src, prepare_aptos_packages, BlockIndex, CompilationCache, TransactionBlock,
        APTOS_COMMONS,
    },
};
use anyhow::{anyhow, bail};
use aptos_framework::natives::code::PackageMetadata;
use aptos_types::transaction::{Transaction, Version};
use clap::Parser;
use move_core_types::account_address::AccountAddress;
use std::{collections::HashMap, path::PathBuf};
use tokio::fs;

#[derive(Parser)]
#[command(about = "Downloads transactions and saves them locally")]
pub struct DownloadCommand {
    #[clap(flatten)]
    rest_api: RestAPI,

    #[clap(
        long,
        help = "Name ofthe file where the downloaded transactions will be saved"
    )]
    transactions_file: String,

    #[clap(
        long,
        help = "path to the folder where the downloaded transactions will be saved"
    )]
    output_dir: Option<PathBuf>,

    #[clap(long, help = "Version of the first transaction to benchmark")]
    begin_version: Version,

    #[clap(
        long,
        help = "End version of transaction range (exclusive) selected for benchmarking"
    )]
    end_version: Version,

    #[clap(long, help = "Collect source code for each transaction")]
    with_source_code: bool,
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

        let output = if let Some(path) = self.output_dir {
            path
        } else {
            PathBuf::from(".")
        };
        if !output.exists() {
            std::fs::create_dir_all(output.as_path()).unwrap();
        }

        let debugger = build_debugger(self.rest_api.rest_endpoint, self.rest_api.api_key)?;

        // Explicitly get transaction corresponding to the end, so we can verify that blocks are
        // fully selected.
        let limit: u64 = self.end_version - self.begin_version + 1;

        let mut is_block_start: bool = true;
        let mut is_block_end: bool = true;
        let length;
        let bytes;
        if self.with_source_code {
            prepare_aptos_packages(output.clone().join(APTOS_COMMONS)).await;
            //let mut index_writer = IndexWriter::new(&output);
            let mut txns_vec = debugger
                .get_full_committed_transactions_with_source_code(
                    self.begin_version,
                    limit,
                    &mut HashMap::new(),
                )
                .await?;
            if !txns_vec[0].1.is_block_start() {
                is_block_start = false;
            }
            if !txns_vec.pop().unwrap().1.is_block_start() {
                is_block_end = false;
            }

            let txn_blocks =
                partition_with_source_code(output.clone(), self.begin_version, txns_vec);
            length = txn_blocks.len();
            bytes = bcs::to_bytes(&txn_blocks).map_err(|err| {
                anyhow!("Error when serializing blocks of transactions: {:?}", err)
            })?;
            //index_writer.dump_version();
            //index_writer.flush_writer();
        } else {
            let (mut txns, _) = debugger
                .get_committed_transactions(self.begin_version, limit)
                .await?;
            if !txns[0].is_block_start() {
                is_block_start = false;
            }
            if !txns.pop().unwrap().is_block_start() {
                is_block_end = false;
            }
            let txn_blocks = partition(self.begin_version, txns);
            length = txn_blocks.len();
            bytes = bcs::to_bytes(&txn_blocks).map_err(|err| {
                anyhow!("Error when serializing blocks of transactions: {:?}", err)
            })?;
        }

        if is_block_start {
            bail!(
                "First transaction {} must be a block start, but it is not",
                self.begin_version
            );
        }
        if is_block_end {
            bail!(
                "All transactions in the block must be selected, transaction {} is not a block \
                end",
                self.end_version - 1
            );
        }

        println!(
            "Downloaded {} blocks with {} transactions in total: versions [{}, {})",
            length, limit, self.begin_version, self.end_version,
        );
        fs::write(output.join(self.transactions_file), &bytes).await?;
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

/// Partitions a sequence of transactions into blocks with source code.
fn partition_with_source_code(
    current_dir: PathBuf,
    begin_version: Version,
    txns: Vec<(
        u64,
        Transaction,
        Option<(
            AccountAddress,
            String,
            HashMap<(AccountAddress, String), PackageMetadata>,
        )>,
    )>,
) -> Vec<BlockIndex> {
    let mut begin_versions_and_blocks = Vec::with_capacity(txns.len());

    let mut curr_begin = begin_version;
    let mut curr_block = Vec::with_capacity(txns.len());
    let mut curr_package_info = HashMap::new();
    let mut compilation_cache = CompilationCache::default();
    let mut package_info_map: HashMap<(AccountAddress, String), Option<u64>> = HashMap::new();
    let mut current_parallel_flag = true;

    for (version, txn, source_code_data) in txns {
        if txn.is_block_start() && !curr_block.is_empty() {
            let block_size = curr_block.len();
            let txn_block = TransactionBlock {
                begin_version: curr_begin,
                transactions: std::mem::take(&mut curr_block),
            };
            let block_idx = BlockIndex {
                transaction_block: txn_block,
                package_info: std::mem::take(&mut curr_package_info),
                _parallel_execution: std::mem::take(&mut current_parallel_flag),
            };
            begin_versions_and_blocks.push(block_idx);
            curr_begin += block_size as Version;
            package_info_map.clear();
        }
        curr_block.push(txn);
        if let Some((address, package_name, map)) = source_code_data {
            let package_info_opt = dump_and_check_src(
                version,
                address,
                package_name.clone(),
                map,
                &mut compilation_cache,
                current_dir.clone(),
            );
            // populate curr_package_info
            if package_info_opt.is_some() {
                let package_info = package_info_opt.unwrap();
                let address_package_name = (address, package_name);
                if package_info_map.contains_key(&address_package_name)
                    && *package_info_map.get(&address_package_name).unwrap()
                        != package_info.upgrade_number
                {
                    current_parallel_flag = false;
                } else {
                    package_info_map.insert(address_package_name, package_info.upgrade_number);
                }
                curr_package_info.insert(version, package_info);
            }
        }
    }
    if !curr_block.is_empty() {
        let txn_block = TransactionBlock {
            begin_version: curr_begin,
            transactions: std::mem::take(&mut curr_block),
        };
        let block_idx = BlockIndex {
            transaction_block: txn_block,
            package_info: std::mem::take(&mut curr_package_info),
            _parallel_execution: std::mem::take(&mut current_parallel_flag),
        };
        begin_versions_and_blocks.push(block_idx);
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
