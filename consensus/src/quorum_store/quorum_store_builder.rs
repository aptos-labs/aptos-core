// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::block_storage::{BlockReader, BlockStore};
use crate::network::NetworkSender;
use crate::payload_manager::PayloadManager;
use crate::quorum_store::batch_coordinator::{BatchCoordinator, BatchCoordinatorCommand};
use crate::quorum_store::batch_generator::{BatchGenerator, BatchGeneratorCommand};
use crate::quorum_store::batch_reader::{BatchReader, BatchReaderCommand};
use crate::quorum_store::batch_store::{BatchStore, BatchStoreCommand};
use crate::quorum_store::direct_mempool_quorum_store::DirectMempoolQuorumStore;
use crate::quorum_store::network_listener::NetworkListener;
use crate::quorum_store::proof_coordinator::{ProofCoordinator, ProofCoordinatorCommand};
use crate::quorum_store::proof_manager::{ProofManager, ProofManagerCommand};
use crate::quorum_store::quorum_store_coordinator::{CoordinatorCommand, QuorumStoreCoordinator};
use crate::quorum_store::quorum_store_db::QuorumStoreDB;
use crate::round_manager::VerifiedEvent;
use aptos_channels::aptos_channel;
use aptos_channels::message_queues::QueueStyle;
use aptos_config::config::{QuorumStoreConfig, SecureBackend};
use aptos_consensus_types::common::Author;
use aptos_consensus_types::request_response::{BlockProposalCommand, CleanCommand, PayloadRequest};
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

    pub fn start(self, block_store: Arc<BlockStore>) -> Option<Sender<CoordinatorCommand>> {
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
    consensus_to_quorum_store_receiver: Receiver<BlockProposalCommand>,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
}

impl DirectMempoolInnerBuilder {
    pub fn new(
        consensus_to_quorum_store_receiver: Receiver<BlockProposalCommand>,
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
    consensus_to_quorum_store_receiver: Receiver<BlockProposalCommand>,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
    aptos_db: Arc<dyn DbReader>,
    network_sender: NetworkSender,
    verifier: ValidatorVerifier,
    backend: SecureBackend,
    coordinator_tx: Sender<CoordinatorCommand>,
    coordinator_rx: Option<Receiver<CoordinatorCommand>>,
    batch_generator_cmd_tx: tokio::sync::mpsc::Sender<BatchGeneratorCommand>,
    batch_generator_cmd_rx: Option<tokio::sync::mpsc::Receiver<BatchGeneratorCommand>>,
    batch_coordinator_cmd_tx: tokio::sync::mpsc::Sender<BatchCoordinatorCommand>,
    batch_coordinator_cmd_rx: Option<tokio::sync::mpsc::Receiver<BatchCoordinatorCommand>>,
    proof_coordinator_cmd_tx: tokio::sync::mpsc::Sender<ProofCoordinatorCommand>,
    proof_coordinator_cmd_rx: Option<tokio::sync::mpsc::Receiver<ProofCoordinatorCommand>>,
    proof_manager_cmd_tx: tokio::sync::mpsc::Sender<ProofManagerCommand>,
    proof_manager_cmd_rx: Option<tokio::sync::mpsc::Receiver<ProofManagerCommand>>,
    batch_store_cmd_tx: tokio::sync::mpsc::Sender<BatchStoreCommand>,
    batch_store_cmd_rx: Option<tokio::sync::mpsc::Receiver<BatchStoreCommand>>,
    batch_reader_cmd_tx: tokio::sync::mpsc::Sender<BatchReaderCommand>,
    batch_reader_cmd_rx: Option<tokio::sync::mpsc::Receiver<BatchReaderCommand>>,
    quorum_store_storage_path: PathBuf,
    num_network_workers_for_fragment: usize,
    quorum_store_storage: Option<Arc<QuorumStoreDB>>,
    quorum_store_msg_tx_vec: Vec<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    quorum_store_msg_rx_vec: Vec<aptos_channel::Receiver<AccountAddress, VerifiedEvent>>,
}

impl InnerBuilder {
    pub fn new(
        epoch: u64,
        author: Author,
        config: QuorumStoreConfig,
        consensus_to_quorum_store_receiver: Receiver<BlockProposalCommand>,
        quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
        aptos_db: Arc<dyn DbReader>,
        network_sender: NetworkSender,
        verifier: ValidatorVerifier,
        backend: SecureBackend,
        quorum_store_storage_path: PathBuf,
        num_network_workers_for_fragment: usize,
    ) -> Self {
        let (coordinator_tx, coordinator_rx) = futures_channel::mpsc::channel(config.channel_size);
        let (batch_generator_cmd_tx, batch_generator_cmd_rx) =
            tokio::sync::mpsc::channel(config.channel_size);
        let (batch_coordinator_cmd_tx, batch_coordinator_cmd_rx) =
            tokio::sync::mpsc::channel(config.channel_size);
        let (proof_coordinator_cmd_tx, proof_coordinator_cmd_rx) =
            tokio::sync::mpsc::channel(config.channel_size);
        let (proof_manager_cmd_tx, proof_manager_cmd_rx) =
            tokio::sync::mpsc::channel(config.channel_size);
        let (batch_store_cmd_tx, batch_store_cmd_rx) =
            tokio::sync::mpsc::channel(config.channel_size);
        let (batch_reader_cmd_tx, batch_reader_cmd_rx) =
            tokio::sync::mpsc::channel(config.channel_size);

        Self {
            epoch,
            author,
            config,
            consensus_to_quorum_store_receiver,
            quorum_store_to_mempool_sender,
            mempool_txn_pull_timeout_ms,
            aptos_db,
            network_sender,
            verifier,
            backend,
            coordinator_tx,
            coordinator_rx: Some(coordinator_rx),
            batch_generator_cmd_tx,
            batch_generator_cmd_rx: Some(batch_generator_cmd_rx),
            batch_coordinator_cmd_tx,
            batch_coordinator_cmd_rx: Some(batch_coordinator_cmd_rx),
            proof_coordinator_cmd_tx,
            proof_coordinator_cmd_rx: Some(proof_coordinator_cmd_rx),
            proof_manager_cmd_tx,
            proof_manager_cmd_rx: Some(proof_manager_cmd_rx),
            batch_store_cmd_tx,
            batch_store_cmd_rx: Some(batch_store_cmd_rx),
            batch_reader_cmd_tx,
            batch_reader_cmd_rx: Some(batch_reader_cmd_rx),
            quorum_store_storage_path,
            num_network_workers_for_fragment,
            quorum_store_storage: None,
            quorum_store_msg_tx_vec: Vec::new(),
            quorum_store_msg_rx_vec: Vec::new(),
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

        // TODO: probably don't need to clear?
        self.quorum_store_msg_tx_vec.clear();
        self.quorum_store_msg_rx_vec.clear();
        for _ in 0..self.num_network_workers_for_fragment + 2 {
            let (quorum_store_msg_tx, quorum_store_msg_rx) =
                aptos_channel::new::<AccountAddress, VerifiedEvent>(
                    QueueStyle::FIFO,
                    self.config.channel_size,
                    None,
                );
            self.quorum_store_msg_tx_vec.push(quorum_store_msg_tx);
            self.quorum_store_msg_rx_vec.push(quorum_store_msg_rx);
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

        let batch_store_cmd_rx = self.batch_store_cmd_rx.take().unwrap();
        let batch_reader_cmd_rx = self.batch_reader_cmd_rx.take().unwrap();
        let (batch_store, batch_reader) = BatchStore::new(
            self.epoch,
            last_committed_round,
            self.author,
            self.network_sender.clone(),
            self.batch_store_cmd_tx.clone(),
            self.batch_reader_cmd_tx.clone(),
            batch_reader_cmd_rx,
            self.quorum_store_storage.as_ref().unwrap().clone(),
            self.verifier.clone(),
            Arc::new(signer),
            self.config.batch_expiry_round_gap_when_init,
            self.config.batch_expiry_round_gap_behind_latest_certified,
            self.config.batch_expiry_round_gap_beyond_latest_certified,
            self.config.batch_expiry_grace_rounds,
            self.config.batch_request_num_peers,
            self.config.batch_request_timeout_ms,
            self.config.memory_quota,
            self.config.db_quota,
        );
        // TODO: batch_reader.start()?
        tokio::spawn(batch_store.start(batch_store_cmd_rx, self.proof_coordinator_cmd_tx.clone()));

        batch_reader
    }

    fn spawn_quorum_store_wrapper(
        mut self,
        block_store: Arc<dyn BlockReader + Send + Sync>,
    ) -> Sender<CoordinatorCommand> {
        let quorum_store_storage = self.quorum_store_storage.as_ref().unwrap().clone();

        // TODO: parameter? bring back back-off?
        let interval = tokio::time::interval(Duration::from_millis(
            self.config.mempool_pulling_interval as u64,
        ));

        let coordinator_rx = self.coordinator_rx.take().unwrap();
        let quorum_store_coordinator = QuorumStoreCoordinator::new(
            self.author,
            self.batch_generator_cmd_tx.clone(),
            self.batch_coordinator_cmd_tx.clone(),
            self.proof_coordinator_cmd_tx.clone(),
            self.proof_manager_cmd_tx.clone(),
            self.batch_store_cmd_tx.clone(),
            self.quorum_store_msg_tx_vec.clone(),
        );
        tokio::spawn(quorum_store_coordinator.start(coordinator_rx));

        let batch_generator_cmd_rx = self.batch_generator_cmd_rx.take().unwrap();
        let batch_generator = BatchGenerator::new(
            self.epoch,
            quorum_store_storage,
            self.quorum_store_to_mempool_sender,
            self.batch_coordinator_cmd_tx.clone(),
            self.mempool_txn_pull_timeout_ms,
            self.config.mempool_txn_pull_max_count,
            self.config.mempool_txn_pull_max_bytes,
            self.config.max_batch_counts,
            self.config.max_batch_bytes,
            self.config.batch_expiry_round_gap_when_init,
            self.config.end_batch_ms,
            self.config.back_pressure_factor * self.verifier.len(),
            block_store,
        );
        tokio::spawn(batch_generator.start(batch_generator_cmd_rx, interval));

        let batch_coordinator_cmd_rx = self.batch_coordinator_cmd_rx.take().unwrap();
        let batch_coordinator = BatchCoordinator::new(
            self.epoch,
            self.author,
            self.network_sender.clone(),
            batch_coordinator_cmd_rx,
            self.batch_store_cmd_tx.clone(),
            self.proof_coordinator_cmd_tx.clone(),
            self.config.max_batch_bytes,
        );
        tokio::spawn(batch_coordinator.start());

        let proof_coordinator_cmd_rx = self.proof_coordinator_cmd_rx.take().unwrap();
        let proof_coordinator = ProofCoordinator::new(self.config.proof_timeout_ms, self.author);
        tokio::spawn(proof_coordinator.start(
            proof_coordinator_cmd_rx,
            self.proof_manager_cmd_tx.clone(),
            self.verifier.clone(),
        ));

        let proof_manager_cmd_rx = self.proof_manager_cmd_rx.take().unwrap();
        let proof_manager = ProofManager::new(self.epoch);
        tokio::spawn(proof_manager.start(
            self.network_sender.clone(),
            self.consensus_to_quorum_store_receiver,
            proof_manager_cmd_rx,
        ));

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

        for network_msg_rx in self.quorum_store_msg_rx_vec.into_iter() {
            let net = NetworkListener::new(
                self.epoch,
                network_msg_rx,
                self.batch_reader_cmd_tx.clone(),
                self.proof_coordinator_cmd_tx.clone(),
                self.batch_coordinator_cmd_tx.clone(),
                self.proof_manager_cmd_tx.clone(),
                self.config.max_batch_bytes,
            );
            tokio::spawn(net.start());
        }

        self.coordinator_tx
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
                Mutex::new(self.coordinator_tx.clone()),
            )),
            self.quorum_store_msg_tx_vec.clone(),
        )
    }

    fn start(self, block_store: Arc<BlockStore>) -> Sender<CoordinatorCommand> {
        self.spawn_quorum_store_wrapper(block_store)
    }
}
