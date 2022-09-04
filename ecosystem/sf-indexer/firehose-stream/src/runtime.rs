// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use aptos_protos::extractor::v1 as extractor;

use crate::convert::convert_transaction;
use aptos_api::context::Context;
use aptos_api_types::{AsConverter, Transaction};
use aptos_config::config::NodeConfig;
use aptos_logger::{debug, error, warn};
use aptos_mempool::MempoolClientSender;
use aptos_types::chain_id::ChainId;
use aptos_vm::data_cache::StorageAdapterOwned;
use extractor::Transaction as TransactionPB;
use futures::channel::mpsc::channel;
use prost::Message;
use std::{convert::TryInto, sync::Arc, time::Duration};
use storage_interface::{state_view::DbStateView, DbReader};
use tokio::{
    runtime::{Builder, Runtime},
    time::sleep,
};

/// Creates a runtime which creates a thread pool which pushes firehose of block protobuf to SF endpoint
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> Option<anyhow::Result<Runtime>> {
    if !config.firehose_stream.enabled {
        return None;
    }

    let runtime = Builder::new_multi_thread()
        .thread_name("fh-stream")
        .disable_lifo_slot()
        .enable_all()
        .build()
        .expect("[fh-stream] failed to create runtime");

    let node_config = config.clone();

    runtime.spawn(async move {
        let context = Context::new(chain_id, db, mp_sender.clone(), node_config.clone());
        let context_arc = Arc::new(context);
        // Let the env variable take precedence over the config file, (if env is not set it just default to 0)
        let config_starting_block = node_config.firehose_stream.starting_block.unwrap_or(0);
        let mut starting_block = std::env::var("STARTING_BLOCK")
            .map(|v| v.parse::<u64>().unwrap_or(0))
            .unwrap_or(0);
        if starting_block == 0 {
            starting_block = config_starting_block;
        }
        let mut streamer = FirehoseStreamer::new(context_arc, starting_block, Some(mp_sender));
        streamer.start().await;
    });
    Some(Ok(runtime))
}

pub struct FirehoseStreamer {
    pub context: Arc<Context>,
    pub resolver: Arc<StorageAdapterOwned<DbStateView>>,
    pub current_block_height: u64,
    pub current_epoch: u64,
    // This is only ever used for testing
    pub mp_sender: MempoolClientSender,
}

impl FirehoseStreamer {
    pub fn new(
        context: Arc<Context>,
        starting_block: u64,
        mp_client_sender: Option<MempoolClientSender>,
    ) -> Self {
        let resolver = Arc::new(context.move_resolver().unwrap());
        let (_block_start_version, _block_last_version, block_event) = context
            .db
            .get_block_info_by_height(starting_block)
            .unwrap_or_else(|_| {
                panic!(
                    "Could not get block_info for starting block {}",
                    starting_block,
                )
            });

        // fake mempool client/sender, if we need to, so we can use the same code for both api and fh-streamer
        let mp_client_sender = mp_client_sender.unwrap_or_else(|| {
            let (mp_client_sender, _mp_client_events) = channel(1);
            mp_client_sender
        });

        Self {
            context,
            resolver,
            current_block_height: block_event.height(),
            current_epoch: block_event.epoch(),
            mp_sender: mp_client_sender,
        }
    }

    pub async fn start(&mut self) {
        // Format is FIRE INIT aptos-node <PACKAGE_VERSION> <MAJOR_VERSION> <MINOR_VERSION> <CHAIN_ID>
        println!(
            "\nFIRE INIT aptos-node {} aptos 0 0 {}",
            env!("CARGO_PKG_VERSION"),
            self.context.chain_id().id(),
        );
        loop {
            self.convert_next_block().await;
        }
    }

    pub async fn convert_next_block(&mut self) -> Vec<TransactionPB> {
        let mut result: Vec<TransactionPB> = vec![];

        let (block_start_version, block_last_version, _) = match self
            .context
            .db
            .get_block_info_by_height(self.current_block_height)
        {
            Ok(block_info) => block_info,
            Err(err) => {
                // TODO: If block has been pruned, panic
                warn!(
                    "[fh-stream] failed to get block info for block_height={}. Error: {}",
                    self.current_block_height, err
                );
                sleep(Duration::from_millis(300)).await;
                return vec![];
            }
        };

        let ledger_info = self.context.get_latest_ledger_info_wrapped().unwrap();
        let block_timestamp = self
            .context
            .db
            .get_block_timestamp(block_start_version)
            .unwrap_or_else(|_| {
                panic!(
                    "Could not get timestamp for version {}",
                    block_start_version
                )
            });
        // We are validating the block as we convert and print each transactions. The rules are as follows:
        // 1. first (and only first) transaction is a block metadata or genesis 2. versions are monotonically increasing 3. start and end versions match block boundaries
        // Retry if the block is not valid. Panic if there's anything wrong with encoding a transaction.
        println!("\nFIRE BLOCK_START {}", self.current_block_height);

        let transactions = match self.context.get_transactions(
            block_start_version,
            (block_last_version - block_start_version + 1)
                .try_into()
                .unwrap(),
            ledger_info.version(),
        ) {
            Ok(transactions) => transactions,
            Err(err) => {
                error!("[fh-stream] failed to get transactions: {}", err);
                sleep(Duration::from_millis(100)).await;
                return vec![];
            }
        };

        if transactions.is_empty() {
            debug!("[fh-stream] no transactions to send");
            sleep(Duration::from_millis(100)).await;
            return vec![];
        }
        debug!(
            "[fh-stream] got {} transactions from {} to {} [version on last actual transaction {}]",
            transactions.len(),
            block_start_version,
            block_last_version,
            transactions.last().map(|txn| txn.version).unwrap_or(0)
        );

        let mut curr_version = block_start_version;
        for onchain_txn in transactions {
            let txn_version = onchain_txn.version;
            let mut txn: Option<Transaction> = None;
            let mut retries = 0;
            while txn.is_none() {
                match self
                    .resolver
                    .as_converter(self.context.db.clone())
                    .try_into_onchain_transaction(block_timestamp, onchain_txn.clone())
                {
                    Ok(transaction) => {
                        txn = Some(transaction);
                    }
                    Err(err) => {
                        if retries == 0 {
                            aptos_logger::debug!(
                                "Could not convert onchain transaction, trying again with updated resolver",
                            );
                        } else {
                            panic!("Could not convert onchain transaction, error: {:?}", err);
                        }
                        retries += 1;
                        self.resolver = Arc::new(self.context.move_resolver().unwrap());
                    }
                }
            }
            let txn = txn.unwrap();
            if !self.validate_transaction_type(curr_version == block_start_version, &txn) {
                error!(
                            "Block {} failed validation: first transaction has to be block metadata or genesis",
                            self.current_block_height
                        );
                sleep(Duration::from_millis(500)).await;
                return vec![];
            }
            if curr_version != txn_version {
                error!(
                    "Block {} failed validation: missing version {}",
                    self.current_block_height, curr_version,
                );
                sleep(Duration::from_millis(500)).await;
                return vec![];
            }
            let txn_proto =
                convert_transaction(&txn, self.current_block_height, self.current_epoch);
            self.print_transaction(&txn_proto);
            result.push(txn_proto);
            curr_version += 1;
        }

        if curr_version - 1 != block_last_version {
            error!(
                "Block {} failed validation: last version supposed to be {} but getting {}",
                self.current_block_height,
                block_last_version,
                curr_version - 1,
            );
            sleep(Duration::from_millis(500)).await;
            return vec![];
        }

        println!("\nFIRE BLOCK_END {}", self.current_block_height);
        metrics::BLOCKS_SENT.inc();
        self.current_block_height += 1;
        result
    }

    /// First, and only first, transaction in a block has to be bmt or genesis
    fn validate_transaction_type(&self, is_first_txn: bool, transaction: &Transaction) -> bool {
        let is_bm_or_genesis = matches!(
            transaction,
            Transaction::BlockMetadataTransaction(_) | Transaction::GenesisTransaction(_)
        );
        is_first_txn == is_bm_or_genesis
    }

    fn print_transaction(&self, transaction: &TransactionPB) {
        let mut buf = vec![];
        transaction.encode(&mut buf).unwrap_or_else(|_| {
            panic!(
                "Could not convert protobuf transaction to bytes '{:?}'",
                transaction
            )
        });
        println!("\nFIRE TRX {}", base64::encode(buf));
        metrics::TRANSACTIONS_SENT.inc();
    }
}
