// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::NetworkSender,
    payload_manager::PayloadManager,
    quorum_store::{
        batch_coordinator::{BatchCoordinator, BatchCoordinatorCommand},
        batch_generator::{BatchGenerator, BatchGeneratorCommand},
        batch_reader::{BatchReader, BatchReaderCommand},
        batch_store::{BatchStore, BatchStoreCommand},
        direct_mempool_quorum_store::DirectMempoolQuorumStore,
        network_listener::NetworkListener,
        proof_coordinator::{ProofCoordinator, ProofCoordinatorCommand},
        proof_manager::{ProofManager, ProofManagerCommand},
        quorum_store_coordinator::{CoordinatorCommand, QuorumStoreCoordinator},
        quorum_store_db::QuorumStoreDB,
    },
    round_manager::VerifiedEvent,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{QuorumStoreConfig, SecureBackend};
use aptos_consensus_types::{common::Author, request_response::GetPayloadCommand};
use aptos_global_constants::CONSENSUS_KEY;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_secure_storage::{KVStorage, Storage};
use aptos_storage_interface::DbReader;
use aptos_types::{
    account_address::AccountAddress, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use futures_channel::mpsc::{Receiver, Sender};
use std::{path::PathBuf, sync::Arc, time::Duration};

pub enum QuorumStoreBuilder {
    DirectMempool(DirectMempoolInnerBuilder),
    QuorumStore(InnerBuilder),
}

impl QuorumStoreBuilder {
    pub fn init_payload_manager(
        &mut self,
    ) -> (
        Arc<PayloadManager>,
        Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    ) {
        match self {
            QuorumStoreBuilder::DirectMempool(inner) => inner.init_payload_manager(),
            QuorumStoreBuilder::QuorumStore(inner) => inner.init_payload_manager(),
        }
    }

    pub fn start(self) -> Option<Sender<CoordinatorCommand>> {
        match self {
            QuorumStoreBuilder::DirectMempool(inner) => {
                inner.start();
                None
            },
            QuorumStoreBuilder::QuorumStore(inner) => Some(inner.start()),
        }
    }
}

pub struct DirectMempoolInnerBuilder {
    consensus_to_quorum_store_receiver: Receiver<GetPayloadCommand>,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
}

impl DirectMempoolInnerBuilder {
    pub fn new(
        consensus_to_quorum_store_receiver: Receiver<GetPayloadCommand>,
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
        Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    ) {
        (Arc::from(PayloadManager::DirectMempool), None)
    }

    fn start(self) {
        let quorum_store = DirectMempoolQuorumStore::new(
            self.consensus_to_quorum_store_receiver,
            self.quorum_store_to_mempool_sender,
            self.mempool_txn_pull_timeout_ms,
        );
        spawn_named!("DirectMempoolQuorumStore", quorum_store.start());
    }
}

// TODO: push most things to config
pub struct InnerBuilder {
    epoch: u64,
    author: Author,
    config: QuorumStoreConfig,
    consensus_to_quorum_store_receiver: Receiver<GetPayloadCommand>,
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
    back_pressure_tx: tokio::sync::mpsc::Sender<bool>,
    back_pressure_rx: Option<tokio::sync::mpsc::Receiver<bool>>,
    quorum_store_storage_path: PathBuf,
    quorum_store_storage: Option<Arc<QuorumStoreDB>>,
    quorum_store_msg_tx: aptos_channel::Sender<AccountAddress, VerifiedEvent>,
    quorum_store_msg_rx: Option<aptos_channel::Receiver<AccountAddress, VerifiedEvent>>,
    remote_batch_coordinator_cmd_tx: Vec<tokio::sync::mpsc::Sender<BatchCoordinatorCommand>>,
    remote_batch_coordinator_cmd_rx: Vec<tokio::sync::mpsc::Receiver<BatchCoordinatorCommand>>,
}

impl InnerBuilder {
    pub fn new(
        epoch: u64,
        author: Author,
        config: QuorumStoreConfig,
        consensus_to_quorum_store_receiver: Receiver<GetPayloadCommand>,
        quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
        aptos_db: Arc<dyn DbReader>,
        network_sender: NetworkSender,
        verifier: ValidatorVerifier,
        backend: SecureBackend,
        quorum_store_storage_path: PathBuf,
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
        let (back_pressure_tx, back_pressure_rx) = tokio::sync::mpsc::channel(config.channel_size);
        let (quorum_store_msg_tx, quorum_store_msg_rx) =
            aptos_channel::new::<AccountAddress, VerifiedEvent>(
                QueueStyle::FIFO,
                config.channel_size,
                None,
            );
        let mut remote_batch_coordinator_cmd_tx = Vec::new();
        let mut remote_batch_coordinator_cmd_rx = Vec::new();
        for _ in 0..config.num_workers_for_remote_fragments {
            let (batch_coordinator_cmd_tx, batch_coordinator_cmd_rx) =
                tokio::sync::mpsc::channel(config.channel_size);
            remote_batch_coordinator_cmd_tx.push(batch_coordinator_cmd_tx);
            remote_batch_coordinator_cmd_rx.push(batch_coordinator_cmd_rx);
        }

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
            back_pressure_tx,
            back_pressure_rx: Some(back_pressure_rx),
            quorum_store_storage_path,
            quorum_store_storage: None,
            quorum_store_msg_tx,
            quorum_store_msg_rx: Some(quorum_store_msg_rx),
            remote_batch_coordinator_cmd_tx,
            remote_batch_coordinator_cmd_rx,
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
        spawn_named!(
            "batch_store",
            batch_store.start(batch_store_cmd_rx, self.proof_coordinator_cmd_tx.clone())
        );

        batch_reader
    }

    fn spawn_quorum_store_wrapper(mut self) -> Sender<CoordinatorCommand> {
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
            self.remote_batch_coordinator_cmd_tx.clone(),
            self.proof_coordinator_cmd_tx.clone(),
            self.proof_manager_cmd_tx.clone(),
            self.batch_store_cmd_tx.clone(),
            self.quorum_store_msg_tx.clone(),
        );
        spawn_named!(
            "quorum_store_coordinator",
            quorum_store_coordinator.start(coordinator_rx)
        );

        let batch_generator_cmd_rx = self.batch_generator_cmd_rx.take().unwrap();
        let back_pressure_rx = self.back_pressure_rx.take().unwrap();
        let batch_generator = BatchGenerator::new(
            self.epoch,
            self.config.clone(),
            quorum_store_storage,
            self.quorum_store_to_mempool_sender,
            self.batch_coordinator_cmd_tx.clone(),
            self.mempool_txn_pull_timeout_ms,
        );
        spawn_named!(
            "batch_generator",
            batch_generator.start(batch_generator_cmd_rx, back_pressure_rx, interval)
        );

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
        spawn_named!("batch_coordinator", batch_coordinator.start());

        for (i, remote_batch_coordinator_cmd_rx) in
            self.remote_batch_coordinator_cmd_rx.into_iter().enumerate()
        {
            let batch_coordinator = BatchCoordinator::new(
                self.epoch,
                self.author,
                self.network_sender.clone(),
                remote_batch_coordinator_cmd_rx,
                self.batch_store_cmd_tx.clone(),
                self.proof_coordinator_cmd_tx.clone(),
                self.config.max_batch_bytes,
            );
            #[allow(unused_variables)]
            let name = format!("batch_coordinator-{}", i).as_str();
            spawn_named!(name, batch_coordinator.start());
        }

        let proof_coordinator_cmd_rx = self.proof_coordinator_cmd_rx.take().unwrap();
        let proof_coordinator = ProofCoordinator::new(self.config.proof_timeout_ms, self.author);
        spawn_named!(
            "proof_coordinator",
            proof_coordinator.start(
                proof_coordinator_cmd_rx,
                self.proof_manager_cmd_tx.clone(),
                self.verifier.clone(),
            )
        );

        let proof_manager_cmd_rx = self.proof_manager_cmd_rx.take().unwrap();
        let proof_manager = ProofManager::new(
            self.epoch,
            self.config.back_pressure.backlog_txn_limit_count,
        );
        spawn_named!(
            "proof_manager",
            proof_manager.start(
                self.network_sender.clone(),
                self.back_pressure_tx.clone(),
                self.consensus_to_quorum_store_receiver,
                proof_manager_cmd_rx,
            )
        );

        let network_msg_rx = self.quorum_store_msg_rx.take().unwrap();
        let net = NetworkListener::new(
            network_msg_rx,
            self.batch_reader_cmd_tx.clone(),
            self.proof_coordinator_cmd_tx.clone(),
            self.remote_batch_coordinator_cmd_tx.clone(),
            self.proof_manager_cmd_tx.clone(),
        );
        spawn_named!("network_listener", net.start());

        self.coordinator_tx
    }

    fn init_payload_manager(
        &mut self,
    ) -> (
        Arc<PayloadManager>,
        Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
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
            Some(self.quorum_store_msg_tx.clone()),
        )
    }

    fn start(self) -> Sender<CoordinatorCommand> {
        self.spawn_quorum_store_wrapper()
    }
}
