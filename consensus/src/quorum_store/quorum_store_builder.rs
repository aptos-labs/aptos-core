// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::block_storage::{BlockReader, BlockStore};
use crate::network::NetworkSender;
use crate::payload_manager::PayloadManager;
use crate::quorum_store::batch_reader::BatchReader;
use crate::quorum_store::direct_mempool_quorum_store::DirectMempoolQuorumStore;
use crate::quorum_store::quorum_store::{QuorumStore, QuorumStoreCommand};
use crate::quorum_store::quorum_store_db::QuorumStoreDB;
use crate::quorum_store::quorum_store_wrapper::QuorumStoreWrapper;
use crate::round_manager::VerifiedEvent;
use aptos_channels::aptos_channel;
use aptos_channels::message_queues::QueueStyle;
use aptos_config::config::{QuorumStoreConfig, SecureBackend};
use aptos_consensus_types::common::Author;
use aptos_consensus_types::request_response::PayloadRequest;
use aptos_global_constants::CONSENSUS_KEY;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_secure_storage::{KVStorage, Storage};
use aptos_storage_interface::DbReader;
use aptos_types::account_address::AccountAddress;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::validator_verifier::ValidatorVerifier;
use futures_channel::mpsc::{Receiver, Sender};
use futures_channel::{mpsc, oneshot};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub enum QuorumStoreBuilder {
    DirectMempool(DirectMempoolInnerBuilder),
    InQuorumStore(InnerBuilder),
}

impl QuorumStoreBuilder {
    pub fn init_payload_manager(
        &mut self,
    ) -> (
        Arc<PayloadManager>,
        Vec<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    ) {
        match self {
            QuorumStoreBuilder::DirectMempool(inner) => inner.init_payload_manager(),
            QuorumStoreBuilder::InQuorumStore(inner) => inner.init_payload_manager(),
        }
    }

    pub fn start(
        self,
        block_store: Arc<BlockStore>,
    ) -> Option<(
        aptos_channel::Sender<AccountAddress, VerifiedEvent>,
        Sender<oneshot::Sender<()>>,
    )> {
        match self {
            QuorumStoreBuilder::DirectMempool(inner) => {
                inner.start();
                None
            },
            QuorumStoreBuilder::InQuorumStore(inner) => Some(inner.start(block_store)),
        }
    }
}

pub struct DirectMempoolInnerBuilder {
    consensus_to_quorum_store_receiver: Receiver<PayloadRequest>,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
}

impl DirectMempoolInnerBuilder {
    pub fn new(
        consensus_to_quorum_store_receiver: Receiver<PayloadRequest>,
        quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
    ) -> Self {
        Self {
            consensus_to_quorum_store_receiver,
            quorum_store_to_mempool_sender,
            mempool_txn_pull_timeout_ms,
        }
    }

    fn init_payload_manager(
        &mut self,
    ) -> (
        Arc<PayloadManager>,
        Vec<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    ) {
        (Arc::from(PayloadManager::DirectMempool), Vec::new())
    }

    fn start(self) {
        let quorum_store = DirectMempoolQuorumStore::new(
            self.consensus_to_quorum_store_receiver,
            self.quorum_store_to_mempool_sender,
            self.mempool_txn_pull_timeout_ms,
        );
        spawn_named!("Quorum Store", quorum_store.start()).unwrap();
    }
}

// TODO: push most things to config
pub struct InnerBuilder {
    epoch: u64,
    author: Author,
    config: QuorumStoreConfig,
    consensus_to_quorum_store_sender: Sender<PayloadRequest>,
    consensus_to_quorum_store_receiver: Receiver<PayloadRequest>,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
    aptos_db: Arc<dyn DbReader>,
    network_sender: NetworkSender,
    verifier: ValidatorVerifier,
    backend: SecureBackend,
    wrapper_command_tx: tokio::sync::mpsc::Sender<QuorumStoreCommand>,
    wrapper_command_rx: Option<tokio::sync::mpsc::Receiver<QuorumStoreCommand>>,
    quorum_store_storage_path: PathBuf,
    num_network_workers_for_fragment: usize,
    quorum_store_storage: Option<Arc<QuorumStoreDB>>,
    quorum_store_msg_tx_vec: Vec<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
}

impl InnerBuilder {
    pub fn new(
        epoch: u64,
        author: Author,
        config: QuorumStoreConfig,
        consensus_to_quorum_store_sender: Sender<PayloadRequest>,
        consensus_to_quorum_store_receiver: Receiver<PayloadRequest>,
        quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
        aptos_db: Arc<dyn DbReader>,
        network_sender: NetworkSender,
        verifier: ValidatorVerifier,
        backend: SecureBackend,
        quorum_store_storage_path: PathBuf,
        num_network_workers_for_fragment: usize,
    ) -> Self {
        let (wrapper_quorum_store_tx, wrapper_quorum_store_rx) =
            tokio::sync::mpsc::channel(config.channel_size);

        Self {
            epoch,
            author,
            config,
            consensus_to_quorum_store_sender,
            consensus_to_quorum_store_receiver,
            quorum_store_to_mempool_sender,
            mempool_txn_pull_timeout_ms,
            aptos_db,
            network_sender,
            verifier,
            backend,
            wrapper_command_tx: wrapper_quorum_store_tx,
            wrapper_command_rx: Some(wrapper_quorum_store_rx),
            quorum_store_storage_path,
            num_network_workers_for_fragment,
            quorum_store_storage: None,
            quorum_store_msg_tx_vec: Vec::new(),
        }
    }

    fn spawn_quorum_store(&mut self) -> Arc<BatchReader> {
        let backend = &self.backend;
        let storage: Storage = backend.try_into().expect("Unable to initialize storage");
        if let Err(error) = storage.available() {
            panic!("Storage is not available: {:?}", error);
        }
        let private_key = storage
            .get(CONSENSUS_KEY)
            .map(|v| v.value)
            .expect("Unable to get private key");
        let signer = ValidatorSigner::new(self.author, private_key);

        let mut quorum_store_msg_rx_vec = Vec::new();
        self.quorum_store_msg_tx_vec.clear();
        for _ in 0..self.num_network_workers_for_fragment + 2 {
            let (quorum_store_msg_tx, quorum_store_msg_rx) =
                aptos_channel::new::<AccountAddress, VerifiedEvent>(
                    QueueStyle::FIFO,
                    self.config.channel_size,
                    None,
                );
            self.quorum_store_msg_tx_vec.push(quorum_store_msg_tx);
            quorum_store_msg_rx_vec.push(quorum_store_msg_rx);
        }

        let latest_ledger_info_with_sigs = self
            .aptos_db
            .get_latest_ledger_info()
            .expect("could not get latest ledger info");
        let last_committed_round = if latest_ledger_info_with_sigs
            .ledger_info()
            .commit_info()
            .epoch()
            == self.epoch
        {
            latest_ledger_info_with_sigs
                .ledger_info()
                .commit_info()
                .round()
        } else {
            0
        };

        let wrapper_command_rx = self.wrapper_command_rx.take();
        let quorum_store_storage = self.quorum_store_storage.clone().unwrap();
        let (quorum_store, batch_reader) = QuorumStore::new(
            self.epoch,
            last_committed_round,
            self.author,
            quorum_store_storage,
            quorum_store_msg_rx_vec,
            self.quorum_store_msg_tx_vec.clone(),
            self.network_sender.clone(),
            self.config.clone(),
            self.verifier.clone(),
            signer,
            wrapper_command_rx.unwrap(),
        );

        let metrics_monitor = tokio_metrics::TaskMonitor::new();
        {
            let metrics_monitor = metrics_monitor.clone();
            tokio::spawn(async move {
                for interval in metrics_monitor.intervals() {
                    println!("QuorumStore:{:?}", interval);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });
        }
        tokio::spawn(quorum_store.start());

        // if let Err(e) = spawn_named!(
        //     &("QuorumStore epoch ".to_owned() + &self.epoch().to_string()),
        //     metrics_monitor.instrument(quorum_store.start())
        // ) {
        //     debug!("QS: spawn_named QuorumStore error {:?}", e);
        // }
        batch_reader
    }

    fn spawn_quorum_store_wrapper(
        self,
        block_store: Arc<dyn BlockReader + Send + Sync>,
    ) -> (
        aptos_channel::Sender<AccountAddress, VerifiedEvent>,
        Sender<oneshot::Sender<()>>,
    ) {
        // TODO: make this not use a ConsensusRequest
        let (wrapper_quorum_store_msg_tx, wrapper_quorum_store_msg_rx) =
            aptos_channel::new::<AccountAddress, VerifiedEvent>(
                QueueStyle::FIFO,
                self.config.channel_size,
                None,
            );

        let (wrapper_shutdown_tx, wrapper_shutdown_rx) = mpsc::channel(0);

        let quorum_store_storage = self.quorum_store_storage.unwrap().clone();
        let quorum_store_wrapper = QuorumStoreWrapper::new(
            self.epoch,
            quorum_store_storage,
            self.quorum_store_to_mempool_sender.clone(),
            self.wrapper_command_tx.clone(),
            self.mempool_txn_pull_timeout_ms,
            self.config.mempool_txn_pull_max_count,
            self.config.mempool_txn_pull_max_bytes,
            self.config.max_batch_counts,
            self.config.max_batch_bytes,
            self.config.batch_expiry_round_gap_when_init,
            self.config.batch_expiry_round_gap_behind_latest_certified,
            self.config.batch_expiry_round_gap_beyond_latest_certified,
            self.config.end_batch_ms,
            self.config.back_pressure_factor * self.verifier.len(),
            block_store,
        );
        let metrics_monitor = tokio_metrics::TaskMonitor::new();
        {
            let metrics_monitor = metrics_monitor.clone();
            tokio::spawn(async move {
                for interval in metrics_monitor.intervals() {
                    println!("QuorumStoreWrapper:{:?}", interval);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });
        }

        // TODO: parameter? bring back back-off?
        let interval = tokio::time::interval(Duration::from_millis(
            self.config.mempool_pulling_interval as u64,
        ));

        tokio::spawn(quorum_store_wrapper.start(
            self.network_sender.clone(),
            self.consensus_to_quorum_store_receiver,
            wrapper_shutdown_rx,
            wrapper_quorum_store_msg_rx,
            interval,
        ));

        (wrapper_quorum_store_msg_tx, wrapper_shutdown_tx)

        // _ = spawn_named!(
        //     &("QuorumStoreWrapper epoch ".to_owned() + &self.epoch().to_string()),
        //     metrics_monitor.instrument(quorum_store_wrapper.start(
        //         network_sender,
        //         consensus_to_quorum_store_rx,
        //         wrapper_shutdown_rx,
        //         wrapper_quorum_store_msg_rx,
        //     ))
        // );
    }

    fn init_payload_manager(
        &mut self,
    ) -> (
        Arc<PayloadManager>,
        Vec<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    ) {
        self.quorum_store_storage = Some(Arc::new(QuorumStoreDB::new(
            self.quorum_store_storage_path.clone(),
        )));

        let batch_reader = self.spawn_quorum_store();

        (
            Arc::from(PayloadManager::InQuorumStore(
                batch_reader,
                // TODO: remove after splitting out clean requests
                Mutex::new(self.consensus_to_quorum_store_sender.clone()),
            )),
            self.quorum_store_msg_tx_vec.clone(),
        )
    }

    fn start(
        self,
        block_store: Arc<BlockStore>,
    ) -> (
        aptos_channel::Sender<AccountAddress, VerifiedEvent>,
        Sender<oneshot::Sender<()>>,
    ) {
        self.spawn_quorum_store_wrapper(block_store)
    }
}
