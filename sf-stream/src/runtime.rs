// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use crate::protos::extractor;

use crate::convert::convert_transaction;
use aptos_api::context::Context;
use aptos_api_types::{AsConverter, Transaction};
use aptos_config::config::NodeConfig;
use aptos_logger::{debug, error, warn};
use aptos_mempool::MempoolClientSender;
use aptos_types::chain_id::ChainId;
use aptos_vm::data_cache::RemoteStorageOwned;
use futures::channel::mpsc::channel;
use protobuf::Message;
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
    mp_sender: MempoolClientSender,
) -> anyhow::Result<Runtime> {
    let runtime = Builder::new_multi_thread()
        .thread_name("sf-stream")
        .enable_all()
        .build()
        .expect("[sf-stream] failed to create runtime");

    let node_config = config.clone();

    // TODO: put this into an arg param
    let starting_version: Option<u64> = None;

    runtime.spawn(async move {
        if node_config.sf_stream.enabled {
            let context = Context::new(chain_id, db, mp_sender.clone(), node_config.clone());
            let context_arc = Arc::new(context);
            let mut streamer = SfStreamer::new(
                node_config.sf_stream.target_address,
                context_arc,
                starting_version.unwrap_or_default(),
                Some(mp_sender),
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
    // This is only ever used for testing
    pub mp_sender: MempoolClientSender,
}

impl SfStreamer {
    pub fn new(
        target_address: SocketAddr,
        context: Arc<Context>,
        starting_version: u64,
        mp_client_sender: Option<MempoolClientSender>,
    ) -> Self {
        let resolver = Arc::new(context.move_resolver().unwrap());
        let latest = context.get_latest_ledger_info().unwrap();
        let block_info = context
            .get_block_info(starting_version, latest.ledger_version.0)
            .unwrap();
        let starting_tnx = context
            .get_transaction_by_version(block_info.start_version, latest.ledger_version.0)
            .unwrap();

        let (version, epoch) = match starting_tnx.transaction {
            aptos_types::transaction::Transaction::BlockMetadata(bmt) => {
                (starting_tnx.version, bmt.epoch())
            }
            aptos_types::transaction::Transaction::GenesisTransaction(_gt) => (0, 0),
            _ => panic!(
                "[sf-stream] first transaction is not a block metadata or genesis transaction"
            ),
        };

        // fake mempool client/sender, if we need to, so we can use the same code for both api and sf-streamer
        let mp_client_sender = mp_client_sender.unwrap_or_else(|| {
            let (mp_client_sender, _mp_client_events) = channel(1);
            mp_client_sender
        });

        Self {
            target_address,
            context,
            current_version: version,
            resolver,
            block_height: block_info.block_height,
            current_epoch: epoch,
            mp_sender: mp_client_sender,
        }
    }

    pub async fn start(&mut self) {
        loop {
            self.batch_convert_once(100).await;
        }
    }

    pub async fn batch_convert_once(&mut self, batch_size: u16) -> Vec<extractor::Transaction> {
        let mut result: Vec<extractor::Transaction> = vec![];
        match &self.context.db.get_first_txn_version() {
            Ok(version_result) => {
                if let Some(oldest_version) = version_result {
                    if oldest_version > &self.current_version {
                        warn!(
                            "[sf-stream] oldest txn version is {} but requested version is {}",
                            oldest_version, &self.current_version
                        );
                        sleep(Duration::from_millis(300)).await;
                        return result;
                    }
                }
            }
            Err(err) => {
                warn!("[sf-stream] failed to get first txn version: {}", err);
                sleep(Duration::from_millis(300)).await;
                return result;
            }
        }

        let ledger_info = self.context.get_latest_ledger_info().unwrap();
        match self
            .context
            .get_transactions(self.current_version, batch_size, ledger_info.version())
        {
            Ok(transactions) => {
                if transactions.is_empty() {
                    debug!("[sf-stream] no transactions to send");
                    sleep(Duration::from_millis(100)).await;
                    return result;
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
                    self.print_transaction(&txn_proto).await;
                    result.push(txn_proto);
                    self.current_version = txn_version;
                }
                self.current_version += 1;
            }
            Err(err) => {
                error!("[sf-stream] failed to get transactions: {}", err);
                sleep(Duration::from_millis(100)).await;
                return result;
            }
        }
        result
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

    pub async fn print_transaction(&self, transaction: &extractor::Transaction) {
        let pb_b64 = &base64::encode(transaction.write_to_bytes().unwrap());
        println!("DMLOG TRX {}", pb_b64);
        metrics::TRANSACTIONS_SENT.inc();
    }
}
