// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use crate::protos::demo;

use aptos_api::context::Context;
use aptos_api_types::TransactionOnChainData;
use aptos_config::config::NodeConfig;
use aptos_logger::{debug, error, warn};
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::Transaction;
use futures::channel::mpsc::channel;
use std::time::Duration;
use std::{net::SocketAddr, sync::Arc};
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
            let streamer = SfStreamer::new(node_config.sf_stream.target_address);
            streamer
                .start(context, starting_version.unwrap_or_default())
                .await;
        }
    });
    Ok(runtime)
}

#[derive(Clone, Debug, PartialEq)]
pub struct SfStreamer {
    pub target_address: SocketAddr,
}

impl SfStreamer {
    pub fn new(target_address: SocketAddr) -> Self {
        Self { target_address }
    }

    pub async fn start(&self, context: Context, starting_version: u64) {
        let mut current_version = starting_version;
        loop {
            match context.db.get_first_txn_version() {
                Ok(version_result) => {
                    if let Some(oldest_version) = version_result {
                        if oldest_version > current_version {
                            warn!(
                                "[sf-stream] oldest txn version is {} but requested version is {}",
                                oldest_version, current_version
                            );
                            sleep(Duration::from_millis(300)).await;
                            continue;
                        }
                    }
                }
                Err(err) => {
                    warn!("[sf-stream] failed to get first txn version: {}", err);
                    sleep(Duration::from_millis(300)).await;
                    continue;
                }
            }

            match context.get_transactions(current_version, 100, current_version) {
                Ok(transactions) => {
                    if transactions.is_empty() {
                        debug!("[sf-stream] no transactions to send");
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    // TODO: there might be an off by one (tx version fetched)
                    debug!(
                        "[sf-stream] got {} transactions from {} to {} [{}]",
                        transactions.len(),
                        current_version,
                        current_version + transactions.len() as u64,
                        transactions.last().map(|txn| txn.version).unwrap_or(0)
                    );
                    for txn in transactions {
                        // TODO: assert txn.version = current_version + 1?
                        // TODO: return a `Result` & check to ensure we pushed before incrementing
                        self.push_transaction(&txn).await;
                        current_version = txn.version;
                    }
                }
                Err(err) => {
                    error!("[sf-stream] failed to get transactions: {}", err);
                    sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
        }
    }

    pub async fn push_transaction(&self, transaction: &TransactionOnChainData) {
        let txn_type = match &transaction.transaction {
            Transaction::UserTransaction(_) => demo::transaction::TransactionType::USER,
            Transaction::GenesisTransaction(_) => demo::transaction::TransactionType::GENESIS,
            Transaction::BlockMetadata(_) => demo::transaction::TransactionType::BLOCK_METADATA,
            Transaction::StateCheckpoint(_) => demo::transaction::TransactionType::STATE_CHECKPOINT,
        };
        let txn_proto = demo::Transaction {
            hash: transaction.info.transaction_hash().to_hex(),
            version: transaction.version,
            type_: protobuf::EnumOrUnknown::new(txn_type),
            special_fields: Default::default(),
        };

        // TODO: Push `transaction` to some server we instantiate
        // send_transaction_proto(txn_proto).await;
        metrics::TRANSACTIONS_SENT.inc();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use aptos_config::config::NodeConfig;
    use aptos_types::chain_id::ChainId;

    use crate::{
        runtime::bootstrap,
        tests::{new_test_context, TestContext},
    };

    #[test]
    fn test_bootstrap_jsonprc_and_api_configured_at_different_port() {
        let mut cfg = NodeConfig::default();
        cfg.randomize_ports();
        bootstrap_with_config(cfg);
    }

    pub fn bootstrap_with_config(cfg: NodeConfig) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let context = runtime.block_on(new_test_context_async(
            "test_bootstrap_jsonprc_and_api_configured_at_different_port",
        ));
        let ret = bootstrap(&cfg, ChainId::test(), context.db.clone());
        assert!(ret.is_ok());

        assert_web_server(cfg.api.address.port());
    }

    pub fn assert_web_server(port: u16) {
        let base_url = format!("http://localhost:{}", port);
        let client = reqwest::blocking::Client::new();
        // first call have retry to ensure the server is ready to serve
        let api_resp = with_retry(|| Ok(client.get(&base_url).send()?)).unwrap();
        assert_eq!(api_resp.status(), 200);
        let healthy_check_resp = client
            .get(format!("{}/-/healthy", base_url))
            .send()
            .unwrap();
        assert_eq!(healthy_check_resp.status(), 200);
    }

    fn with_retry<F>(f: F) -> anyhow::Result<reqwest::blocking::Response>
    where
        F: Fn() -> anyhow::Result<reqwest::blocking::Response>,
    {
        let mut remaining_attempts = 60;
        loop {
            match f() {
                Ok(r) => return Ok(r),
                Err(_) if remaining_attempts > 0 => {
                    remaining_attempts -= 1;
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub async fn new_test_context_async(test_name: &'static str) -> TestContext {
        new_test_context(test_name)
    }
}
