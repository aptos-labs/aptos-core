// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::quorum_store_db::QuorumStoreStorage;
use crate::{
    consensus_observer::publisher::consensus_publisher::ConsensusPublisher,
    error::error_kind,
    network::{IncomingBatchRetrievalRequest, NetworkSender},
    network_interface::ConsensusMsg,
    payload_manager::{DirectMempoolPayloadManager, QuorumStorePayloadManager, TPayloadManager},
    quorum_store::{
        batch_coordinator::{BatchCoordinator, BatchCoordinatorCommand},
        batch_generator::{BackPressure, BatchGenerator, BatchGeneratorCommand},
        batch_requester::BatchRequester,
        batch_store::{BatchReader, BatchReaderImpl, BatchStore},
        counters,
        direct_mempool_quorum_store::DirectMempoolQuorumStore,
        network_listener::NetworkListener,
        proof_coordinator::{ProofCoordinator, ProofCoordinatorCommand},
        proof_manager::{ProofManager, ProofManagerCommand},
        quorum_store_coordinator::{CoordinatorCommand, QuorumStoreCoordinator},
        types::{Batch, BatchResponse},
    },
    round_manager::VerifiedEvent,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{QuorumStoreConfig, SecureBackend};
use aptos_consensus_types::{
    common::Author, proof_of_store::ProofCache, request_response::GetPayloadCommand,
};
use aptos_global_constants::CONSENSUS_KEY;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_secure_storage::{KVStorage, Storage};
use aptos_storage_interface::DbReader;
use aptos_types::{
    account_address::AccountAddress, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use futures::StreamExt;
use futures_channel::mpsc::{Receiver, Sender};
use std::{sync::Arc, time::Duration};

pub enum QuorumStoreBuilder {
    DirectMempool(DirectMempoolInnerBuilder),
    QuorumStore(InnerBuilder),
}

impl QuorumStoreBuilder {
    pub fn init_payload_manager(
        &mut self,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ) -> (
        Arc<dyn TPayloadManager>,
        Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    ) {
        match self {
            QuorumStoreBuilder::DirectMempool(inner) => inner.init_payload_manager(),
            QuorumStoreBuilder::QuorumStore(inner) => {
                inner.init_payload_manager(consensus_publisher)
            },
        }
    }

    pub fn start(
        self,
    ) -> Option<(
        Sender<CoordinatorCommand>,
        aptos_channel::Sender<AccountAddress, IncomingBatchRetrievalRequest>,
    )> {
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
        Arc<dyn TPayloadManager>,
        Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    ) {
        (Arc::from(DirectMempoolPayloadManager::new()), None)
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
    num_validators: u64,
    config: QuorumStoreConfig,
    consensus_to_quorum_store_receiver: Receiver<GetPayloadCommand>,
    quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
    mempool_txn_pull_timeout_ms: u64,
    aptos_db: Arc<dyn DbReader>,
    network_sender: NetworkSender,
    verifier: ValidatorVerifier,
    proof_cache: ProofCache,
    backend: SecureBackend,
    coordinator_tx: Sender<CoordinatorCommand>,
    coordinator_rx: Option<Receiver<CoordinatorCommand>>,
    batch_generator_cmd_tx: tokio::sync::mpsc::Sender<BatchGeneratorCommand>,
    batch_generator_cmd_rx: Option<tokio::sync::mpsc::Receiver<BatchGeneratorCommand>>,
    proof_coordinator_cmd_tx: tokio::sync::mpsc::Sender<ProofCoordinatorCommand>,
    proof_coordinator_cmd_rx: Option<tokio::sync::mpsc::Receiver<ProofCoordinatorCommand>>,
    proof_manager_cmd_tx: tokio::sync::mpsc::Sender<ProofManagerCommand>,
    proof_manager_cmd_rx: Option<tokio::sync::mpsc::Receiver<ProofManagerCommand>>,
    back_pressure_tx: tokio::sync::mpsc::Sender<BackPressure>,
    back_pressure_rx: Option<tokio::sync::mpsc::Receiver<BackPressure>>,
    quorum_store_storage: Arc<dyn QuorumStoreStorage>,
    quorum_store_msg_tx: aptos_channel::Sender<AccountAddress, VerifiedEvent>,
    quorum_store_msg_rx: Option<aptos_channel::Receiver<AccountAddress, VerifiedEvent>>,
    remote_batch_coordinator_cmd_tx: Vec<tokio::sync::mpsc::Sender<BatchCoordinatorCommand>>,
    remote_batch_coordinator_cmd_rx: Vec<tokio::sync::mpsc::Receiver<BatchCoordinatorCommand>>,
    batch_store: Option<Arc<BatchStore>>,
    batch_reader: Option<Arc<dyn BatchReader>>,
    broadcast_proofs: bool,
}

impl InnerBuilder {
    pub(crate) fn new(
        epoch: u64,
        author: Author,
        num_validators: u64,
        config: QuorumStoreConfig,
        consensus_to_quorum_store_receiver: Receiver<GetPayloadCommand>,
        quorum_store_to_mempool_sender: Sender<QuorumStoreRequest>,
        mempool_txn_pull_timeout_ms: u64,
        aptos_db: Arc<dyn DbReader>,
        network_sender: NetworkSender,
        verifier: ValidatorVerifier,
        proof_cache: ProofCache,
        backend: SecureBackend,
        quorum_store_storage: Arc<dyn QuorumStoreStorage>,
        broadcast_proofs: bool,
    ) -> Self {
        let (coordinator_tx, coordinator_rx) = futures_channel::mpsc::channel(config.channel_size);
        let (batch_generator_cmd_tx, batch_generator_cmd_rx) =
            tokio::sync::mpsc::channel(config.channel_size);
        let (proof_coordinator_cmd_tx, proof_coordinator_cmd_rx) =
            tokio::sync::mpsc::channel(config.channel_size);
        let (proof_manager_cmd_tx, proof_manager_cmd_rx) =
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
        for _ in 0..config.num_workers_for_remote_batches {
            let (batch_coordinator_cmd_tx, batch_coordinator_cmd_rx) =
                tokio::sync::mpsc::channel(config.channel_size);
            remote_batch_coordinator_cmd_tx.push(batch_coordinator_cmd_tx);
            remote_batch_coordinator_cmd_rx.push(batch_coordinator_cmd_rx);
        }

        Self {
            epoch,
            author,
            num_validators,
            config,
            consensus_to_quorum_store_receiver,
            quorum_store_to_mempool_sender,
            mempool_txn_pull_timeout_ms,
            aptos_db,
            network_sender,
            verifier,
            proof_cache,
            backend,
            coordinator_tx,
            coordinator_rx: Some(coordinator_rx),
            batch_generator_cmd_tx,
            batch_generator_cmd_rx: Some(batch_generator_cmd_rx),
            proof_coordinator_cmd_tx,
            proof_coordinator_cmd_rx: Some(proof_coordinator_cmd_rx),
            proof_manager_cmd_tx,
            proof_manager_cmd_rx: Some(proof_manager_cmd_rx),
            back_pressure_tx,
            back_pressure_rx: Some(back_pressure_rx),
            quorum_store_storage,
            quorum_store_msg_tx,
            quorum_store_msg_rx: Some(quorum_store_msg_rx),
            remote_batch_coordinator_cmd_tx,
            remote_batch_coordinator_cmd_rx,
            batch_store: None,
            batch_reader: None,
            broadcast_proofs,
        }
    }

    fn create_batch_store(&mut self) -> Arc<BatchReaderImpl<NetworkSender>> {
        let backend = &self.backend;
        let storage: Storage = backend.into();
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
        let last_committed_timestamp = latest_ledger_info_with_sigs.commit_info().timestamp_usecs();

        let batch_requester = BatchRequester::new(
            self.epoch,
            self.author,
            self.config.batch_request_num_peers,
            self.config.batch_request_retry_limit,
            self.config.batch_request_retry_interval_ms,
            self.config.batch_request_rpc_timeout_ms,
            self.network_sender.clone(),
            self.verifier.clone(),
        );
        let batch_store = Arc::new(BatchStore::new(
            self.epoch,
            last_committed_timestamp,
            self.quorum_store_storage.clone(),
            self.config.memory_quota,
            self.config.db_quota,
            self.config.batch_quota,
            signer,
        ));
        self.batch_store = Some(batch_store.clone());
        let batch_reader = Arc::new(BatchReaderImpl::new(batch_store.clone(), batch_requester));
        self.batch_reader = Some(batch_reader.clone());

        batch_reader
    }

    #[allow(clippy::unwrap_used)]
    fn spawn_quorum_store(
        mut self,
    ) -> (
        Sender<CoordinatorCommand>,
        aptos_channel::Sender<AccountAddress, IncomingBatchRetrievalRequest>,
    ) {
        // TODO: parameter? bring back back-off?
        let interval = tokio::time::interval(Duration::from_millis(
            self.config.batch_generation_poll_interval_ms as u64,
        ));

        let coordinator_rx = self.coordinator_rx.take().unwrap();
        let quorum_store_coordinator = QuorumStoreCoordinator::new(
            self.author,
            self.batch_generator_cmd_tx.clone(),
            self.remote_batch_coordinator_cmd_tx.clone(),
            self.proof_coordinator_cmd_tx.clone(),
            self.proof_manager_cmd_tx.clone(),
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
            self.author,
            self.config.clone(),
            self.quorum_store_storage.clone(),
            self.batch_store.clone().unwrap(),
            self.quorum_store_to_mempool_sender,
            self.mempool_txn_pull_timeout_ms,
        );
        spawn_named!(
            "batch_generator",
            batch_generator.start(
                self.network_sender.clone(),
                batch_generator_cmd_rx,
                back_pressure_rx,
                interval
            )
        );

        for (i, remote_batch_coordinator_cmd_rx) in
            self.remote_batch_coordinator_cmd_rx.into_iter().enumerate()
        {
            let batch_coordinator = BatchCoordinator::new(
                self.author,
                self.network_sender.clone(),
                self.proof_manager_cmd_tx.clone(),
                self.batch_generator_cmd_tx.clone(),
                self.batch_store.clone().unwrap(),
                self.config.receiver_max_batch_txns as u64,
                self.config.receiver_max_batch_bytes as u64,
                self.config.receiver_max_total_txns as u64,
                self.config.receiver_max_total_bytes as u64,
            );
            #[allow(unused_variables)]
            let name = format!("batch_coordinator-{}", i);
            spawn_named!(
                name.as_str(),
                batch_coordinator.start(remote_batch_coordinator_cmd_rx)
            );
        }

        let proof_coordinator_cmd_rx = self.proof_coordinator_cmd_rx.take().unwrap();
        let proof_coordinator = ProofCoordinator::new(
            self.config.proof_timeout_ms,
            self.author,
            self.batch_reader.clone().unwrap(),
            self.batch_generator_cmd_tx.clone(),
            self.proof_cache,
            self.broadcast_proofs,
        );
        spawn_named!(
            "proof_coordinator",
            proof_coordinator.start(
                proof_coordinator_cmd_rx,
                self.network_sender.clone(),
                self.verifier.clone(),
            )
        );

        let proof_manager_cmd_rx = self.proof_manager_cmd_rx.take().unwrap();
        let proof_manager = ProofManager::new(
            self.author,
            self.config.back_pressure.backlog_txn_limit_count,
            self.config
                .back_pressure
                .backlog_per_validator_batch_limit_count
                * self.num_validators,
            self.batch_store.clone().unwrap(),
            self.config.allow_batches_without_pos_in_proposal,
            self.config.enable_opt_quorum_store,
        );
        spawn_named!(
            "proof_manager",
            proof_manager.start(
                self.back_pressure_tx.clone(),
                self.consensus_to_quorum_store_receiver,
                proof_manager_cmd_rx,
            )
        );

        let network_msg_rx = self.quorum_store_msg_rx.take().unwrap();
        let net = NetworkListener::new(
            network_msg_rx,
            self.proof_coordinator_cmd_tx.clone(),
            self.remote_batch_coordinator_cmd_tx.clone(),
            self.proof_manager_cmd_tx.clone(),
        );
        spawn_named!("network_listener", net.start());

        let batch_store = self.batch_store.clone().unwrap();
        let epoch = self.epoch;
        let (batch_retrieval_tx, mut batch_retrieval_rx) =
            aptos_channel::new::<AccountAddress, IncomingBatchRetrievalRequest>(
                QueueStyle::LIFO,
                10,
                Some(&counters::BATCH_RETRIEVAL_TASK_MSGS),
            );
        let aptos_db_clone = self.aptos_db.clone();
        spawn_named!("batch_serve", async move {
            info!(epoch = epoch, "Batch retrieval task starts");
            while let Some(rpc_request) = batch_retrieval_rx.next().await {
                counters::RECEIVED_BATCH_REQUEST_COUNT.inc();
                let response = if let Ok(value) =
                    batch_store.get_batch_from_local(&rpc_request.req.digest())
                {
                    let batch: Batch = value.try_into().unwrap();
                    BatchResponse::Batch(batch)
                } else {
                    match aptos_db_clone.get_latest_ledger_info() {
                        Ok(ledger_info) => BatchResponse::NotFound(ledger_info),
                        Err(e) => {
                            let e = anyhow::Error::from(e);
                            error!(epoch = epoch, error = ?e, kind = error_kind(&e));
                            continue;
                        },
                    }
                };

                let msg = ConsensusMsg::BatchResponseV2(Box::new(response));
                let bytes = rpc_request.protocol.to_bytes(&msg).unwrap();
                if let Err(e) = rpc_request
                    .response_sender
                    .send(Ok(bytes.into()))
                    .map_err(|_| anyhow::anyhow!("Failed to send block retrieval response"))
                {
                    warn!(epoch = epoch, error = ?e, kind = error_kind(&e));
                }
            }
            info!(epoch = epoch, "Batch retrieval task stops");
        });

        (self.coordinator_tx, batch_retrieval_tx)
    }

    fn init_payload_manager(
        &mut self,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ) -> (
        Arc<dyn TPayloadManager>,
        Option<aptos_channel::Sender<AccountAddress, VerifiedEvent>>,
    ) {
        let batch_reader = self.create_batch_store();

        (
            Arc::from(QuorumStorePayloadManager::new(
                batch_reader,
                // TODO: remove after splitting out clean requests
                self.coordinator_tx.clone(),
                consensus_publisher,
                self.verifier.get_ordered_account_addresses(),
            )),
            Some(self.quorum_store_msg_tx.clone()),
        )
    }

    fn start(
        self,
    ) -> (
        Sender<CoordinatorCommand>,
        aptos_channel::Sender<AccountAddress, IncomingBatchRetrievalRequest>,
    ) {
        self.spawn_quorum_store()
    }
}
