// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use crate::protos::extractor;

use crate::convert::convert_transaction;
use aptos_api::context::Context;
use aptos_api_types::{AsConverter, Transaction};
use aptos_config::config::NodeConfig;
use aptos_logger::{debug, error, warn};
use aptos_types::{chain_id::ChainId, transaction::Transaction::BlockMetadata};
use aptos_vm::data_cache::RemoteStorageOwned;
use futures::channel::mpsc::channel;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
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
) -> anyhow::Result<Runtime> {
    let runtime = Builder::new_multi_thread()
        .thread_name("sf-stream")
        .enable_all()
        .build()
        .expect("[sf-stream] failed to create runtime");

    let node_config = config.clone();

    // TODO: put this into an arg param
    let starting_version: Option<u64> = None;

    // fake mempool client/sender, so we can use the same code for both api and sf-streamer
    let (mp_client_sender, _mp_client_events) = channel(1);

    runtime.spawn(async move {
        if node_config.sf_stream.enabled {
            let context = Context::new(chain_id, db, mp_client_sender, node_config.clone());
            let context_arc = Arc::new(context);
            let mut streamer = SfStreamer::new(
                node_config.sf_stream.target_address,
                context_arc,
                starting_version.unwrap_or_default(),
            );
            streamer.start().await;
        }
    });
    Ok(runtime)
}

pub struct SfStreamer {
    pub target_address: SocketAddr,
    pub context: Arc<Context>,
    pub current_version: u64,
    pub resolver: Arc<RemoteStorageOwned<DbStateView>>,
    pub block_height: u64,
    pub current_epoch: u64,
}

impl SfStreamer {
    pub fn new(target_address: SocketAddr, context: Arc<Context>, starting_version: u64) -> Self {
        let resolver = Arc::new(context.move_resolver().unwrap());
        let latest = context.get_latest_ledger_info().unwrap();
        let block_info = context
            .get_block_info(starting_version, latest.ledger_version.0)
            .unwrap();
        let block_metadata = context
            .get_transaction_by_version(block_info.start_version, latest.ledger_version.0)
            .unwrap();

        if let BlockMetadata(bmt) = block_metadata.transaction {
            return Self {
                target_address,
                context,
                current_version: block_metadata.version,
                resolver,
                block_height: block_info.block_height,
                current_epoch: bmt.epoch(),
            };
        } else {
            panic!("block_metadata is not a block metadata");
        }
    }

    pub async fn start(&mut self) {
        loop {
            self.batch_convert_once(100).await;
        }
    }

    pub async fn batch_convert_once(&mut self, batch_size: u16) {
        match &self.context.db.get_first_txn_version() {
            Ok(version_result) => {
                if let Some(oldest_version) = version_result {
                    if oldest_version > &self.current_version {
                        warn!(
                            "[sf-stream] oldest txn version is {} but requested version is {}",
                            oldest_version, &self.current_version
                        );
                        sleep(Duration::from_millis(300)).await;
                        return;
                    }
                }
            }
            Err(err) => {
                warn!("[sf-stream] failed to get first txn version: {}", err);
                sleep(Duration::from_millis(300)).await;
                return;
            }
        }

        match self
            .context
            .get_transactions(self.current_version, batch_size, self.current_version)
        {
            Ok(transactions) => {
                if transactions.is_empty() {
                    debug!("[sf-stream] no transactions to send");
                    sleep(Duration::from_millis(100)).await;
                    return;
                }
                // TODO: there might be an off by one (tx version fetched)
                debug!(
                    "[sf-stream] got {} transactions from {} to {} [{}]",
                    transactions.len(),
                    self.current_version,
                    self.current_version + transactions.len() as u64,
                    transactions.last().map(|txn| txn.version).unwrap_or(0)
                );
                for onchain_txn in transactions {
                    // TODO: assert txn.version == &self.current_version + 1?
                    // TODO: return a `Result` & check to ensure we pushed before incrementing
                    let txn_version = onchain_txn.version;
                    // Todo: since the timestamp is per block, only calculate this value once per block
                    let timestamp = self
                        .context
                        .get_block_timestamp(onchain_txn.version)
                        .unwrap();
                    let txn = self
                        .resolver
                        .as_converter()
                        .try_into_onchain_transaction(timestamp, onchain_txn)
                        .unwrap();

                    let txn_proto = self.convert_transaction(txn);
                    self.push_transaction(txn_proto).await;
                    self.current_version = txn_version;
                }
            }
            Err(err) => {
                error!("[sf-stream] failed to get transactions: {}", err);
                sleep(Duration::from_millis(100)).await;
                return;
            }
        };
    }

    pub fn maybe_update_from_block_metadata(&mut self, transaction: &Transaction) {
        if let Transaction::BlockMetadataTransaction(bmt) = transaction {
            // TODO: ADD BLOCK HEIGHT UPDATES ONCE ITS IN BMT
            self.block_height += 1;
            self.current_epoch = bmt.epoch.0
        }
    }

    pub fn convert_transaction(&mut self, transaction: Transaction) -> extractor::Transaction {
        self.maybe_update_from_block_metadata(&transaction);
        convert_transaction(&transaction, self.block_height, self.current_epoch)
    }

    pub async fn push_transaction(&self, _transaction: extractor::Transaction) {
        // TODO: Push `transaction` to some server we instantiate
        metrics::TRANSACTIONS_QUEUED.inc();
        // &self.send_transaction_proto(txn_proto).await;
        metrics::TRANSACTIONS_SENT.inc();
    }
}
