// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use aptos_protos::extractor::v1 as extractor;

use crate::convert::convert_transaction;
use anyhow::{bail, ensure};
use aptos_api::context::Context;
use aptos_api_types::{AsConverter, Transaction};
use aptos_config::config::NodeConfig;
use aptos_logger::{debug, error, warn};
use aptos_mempool::MempoolClientSender;
use aptos_types::chain_id::ChainId;
use aptos_vm::data_cache::RemoteStorageOwned;
use extractor::{transaction::TransactionType, Transaction as TransactionPB};
use futures::channel::mpsc::channel;
use prost::Message;
use std::convert::TryInto;
use std::sync::Arc;
use std::time::Duration;
use storage_interface::state_view::DbStateView;
use storage_interface::DbReader;
use tokio::runtime::{Builder, Runtime};
use tokio::time::sleep;

/// Creates a runtime which creates a thread pool which pushes firehose of block protobuf to SF endpoint
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> Option<anyhow::Result<Runtime>> {
    if !config.sf_stream.enabled {
        return None;
    }

    let runtime = Builder::new_multi_thread()
        .thread_name("sf-stream")
        .enable_all()
        .build()
        .expect("[sf-stream] failed to create runtime");

    let node_config = config.clone();

    runtime.spawn(async move {
        let context = Context::new(chain_id, db, mp_sender.clone(), node_config.clone());
        let context_arc = Arc::new(context);
        // Let the env variable take precedence over the config file
        let config_starting_version = node_config.sf_stream.starting_version.unwrap_or(0);
        let starting_version = std::env::var("STARTING_VERSION")
            .map(|v| v.parse::<u64>().unwrap_or(config_starting_version))
            .unwrap_or(config_starting_version);

        let mut streamer = SfStreamer::new(context_arc, starting_version, Some(mp_sender));
        streamer.start().await;
    });
    Some(Ok(runtime))
}

pub struct SfStreamer {
    pub context: Arc<Context>,
    pub resolver: Arc<RemoteStorageOwned<DbStateView>>,
    pub current_block_height: u64,
    pub current_epoch: u64,
    // This is only ever used for testing
    pub mp_sender: MempoolClientSender,
}

impl SfStreamer {
    pub fn new(
        context: Arc<Context>,
        starting_version: u64,
        mp_client_sender: Option<MempoolClientSender>,
    ) -> Self {
        let resolver = Arc::new(context.move_resolver().unwrap());
        let (_block_start_version, _block_last_versionn, block_event) = context
            .db
            .get_block_info(starting_version)
            .unwrap_or_else(|_| {
                panic!(
                    "Could not get block_info for starting version {}",
                    starting_version,
                )
            });

        // fake mempool client/sender, if we need to, so we can use the same code for both api and sf-streamer
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
        println!("FIRE INIT aptos-node {} aptos 0 0", env!("CARGO_PKG_VERSION"));
        loop {
            self.convert_next_block().await;
        }
    }

    pub async fn convert_next_block(&mut self) -> Vec<TransactionPB> {
        let mut result: Vec<TransactionPB> = vec![];
        let block_start_version;
        let block_last_version;
        match self
            .context
            .db
            .get_block_info_by_height(self.current_block_height)
        {
            Ok(block_info) => {
                (block_start_version, block_last_version, _) = block_info;
            }
            Err(err) => {
                // TODO: If block has been pruned, panic
                warn!(
                    "[sf-stream] failed to get block info for block_height={}. Error: {}",
                    self.current_block_height, err
                );
                sleep(Duration::from_millis(300)).await;
                return vec![];
            }
        }

        let ledger_info = self.context.get_latest_ledger_info().unwrap();
        match self.context.get_transactions(
            block_start_version,
            (block_last_version - block_start_version + 1)
                .try_into()
                .unwrap(),
            ledger_info.version(),
        ) {
            Ok(transactions) => {
                if transactions.is_empty() {
                    debug!("[sf-stream] no transactions to send");
                    sleep(Duration::from_millis(100)).await;
                    return vec![];
                }
                debug!(
                    "[sf-stream] got {} transactions from {} to {} [version on last actual transaction {}]",
                    transactions.len(),
                    block_start_version,
                    block_last_version,
                    transactions.last().map(|txn| txn.version).unwrap_or(0)
                );
                let mut block_timestamp = None;
                for onchain_txn in transactions {
                    if block_timestamp.is_none() {
                        block_timestamp = Some(
                            self.context
                                .get_block_timestamp(onchain_txn.version)
                                .unwrap_or_else(|_| {
                                    panic!(
                                        "Could not get timestamp for version {}",
                                        onchain_txn.version
                                    )
                                }),
                        );
                    }
                    let txn_version = onchain_txn.version;
                    let txn = self
                        .resolver
                        .as_converter(self.context.db.clone())
                        .try_into_onchain_transaction(block_timestamp.unwrap(), onchain_txn)
                        .unwrap_or_else(|e| {
                            panic!(
                                "Could not convert onchain transaction version {} into transaction: {:?}",
                                txn_version, e
                            )
                        });

                    let txn_proto = self.convert_transaction(txn);
                    result.push(txn_proto);
                }
            }
            Err(err) => {
                error!("[sf-stream] failed to get transactions: {}", err);
                sleep(Duration::from_millis(100)).await;
                return vec![];
            }
        }
        match self.print_block_with_validation(&result, block_start_version, block_last_version) {
            Ok(_) => {
                self.current_block_height += 1;
                result
            }
            Err(err) => {
                error!("[sf-stream] Validation failed: {}", err);
                sleep(Duration::from_millis(500)).await;
                vec![]
            }
        }
    }

    pub fn convert_transaction(&self, transaction: Transaction) -> TransactionPB {
        convert_transaction(&transaction, self.current_block_height, self.current_epoch)
    }

    /// We can consider a block height as valid if these conditions are met:
    /// 1. first (and only first) transaction is a block metadata or genesis 2. versions are monotonically increasing 3. start and end versions match block boundaries
    /// Return error if the block is not valid. Panic if there's anything wrong with encoding a transaction.
    fn print_block_with_validation(
        &self,
        converted_txns: &Vec<TransactionPB>,
        block_start_version: u64,
        block_last_version: u64,
    ) -> anyhow::Result<()> {
        if converted_txns.is_empty() {
            bail!("No transactions")
        }
        println!("FIRE BLOCK_START {}", self.current_block_height);
        let mut curr_version = block_start_version;
        for (index, txn) in converted_txns.iter().enumerate() {
            // First, and only first, transaction has to be bmt or genesis
            let is_bm_or_genesis = match txn.r#type() {
                TransactionType::BlockMetadata => true,
                TransactionType::Genesis => true,
                TransactionType::User => false,
                TransactionType::StateCheckpoint => false,
            };
            if index == 0 {
                ensure!(
                    is_bm_or_genesis,
                    "First transaction has to be block metadata for block {}, found {}",
                    self.current_block_height,
                    txn.r#type
                );
            } else {
                ensure!(
                    !is_bm_or_genesis,
                    "Multiple {} detected for block {}",
                    txn.r#type,
                    self.current_block_height
                );
            }
            // Start version has to be first version and versions have to be monotonically increasing
            ensure!(
                curr_version == txn.version,
                "Missing version {} for block {}",
                block_start_version,
                self.current_block_height
            );
            self.print_transaction(txn);
            curr_version += 1
        }
        // Last version has to match last version of transaction
        ensure!(
            curr_version - 1 == block_last_version,
            "Last version supposed to be {} but getting {} for block {}",
            block_start_version,
            curr_version - 1,
            self.current_block_height
        );
        println!("FIRE BLOCK_END {}", self.current_block_height);
        metrics::BLOCKS_SENT.inc();
        Ok(())
    }

    fn print_transaction(&self, transaction: &TransactionPB) {
        let mut buf = vec![];
        transaction.encode(&mut buf).unwrap_or_else(|_| {
            panic!(
                "Could not convert protobuf transaction to bytes '{:?}'",
                transaction
            )
        });
        println!("FIRE TRX {}", base64::encode(buf));
        metrics::TRANSACTIONS_SENT.inc();
    }
}
