// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    quorum_store::{
        batch_reader::{BatchReader, BatchReaderCommand},
        counters,
        proof_coordinator::ProofCoordinatorCommand,
        quorum_store_db::QuorumStoreDB,
        types::{Batch, PersistedValue},
    },
};
// use aptos_logger::spawn_named;
use aptos_consensus_types::{
    common::Round,
    proof_of_store::{LogicalTime, SignedDigest},
};
use aptos_crypto::HashValue;
use aptos_logger::debug;
use aptos_types::{
    transaction::SignedTransaction, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier, PeerId,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PersistRequest {
    digest: HashValue,
    value: PersistedValue,
}

impl PersistRequest {
    pub fn new(
        author: PeerId,
        payload: Vec<SignedTransaction>,
        digest_hash: HashValue,
        num_bytes: usize,
        expiration: LogicalTime,
    ) -> Self {
        Self {
            digest: digest_hash,
            value: PersistedValue::new(Some(payload), expiration, author, num_bytes),
        }
    }
}

#[derive(Debug)]
pub(crate) enum BatchStoreCommand {
    Persist(PersistRequest),
    BatchRequest(
        HashValue,
        PeerId,
        Option<oneshot::Sender<Result<Vec<SignedTransaction>, aptos_executor_types::Error>>>,
    ),
    Clean(Vec<HashValue>),
    Shutdown(oneshot::Sender<()>),
}

pub(crate) struct BatchStore<T> {
    epoch: u64,
    my_peer_id: PeerId,
    network_sender: T,
    batch_reader: Arc<BatchReader>,
    db: Arc<QuorumStoreDB>,
    validator_signer: Arc<ValidatorSigner>,
}

// TODO: send config to reduce number of arguments?
#[allow(clippy::too_many_arguments)]
impl<T: QuorumStoreSender + Clone + Send + Sync + 'static> BatchStore<T> {
    pub fn new(
        epoch: u64,
        last_committed_round: Round,
        my_peer_id: PeerId,
        network_sender: T,
        batch_store_tx: Sender<BatchStoreCommand>,
        batch_reader_tx: Sender<BatchReaderCommand>,
        batch_reader_rx: Receiver<BatchReaderCommand>,
        db: Arc<QuorumStoreDB>,
        validator_verifier: ValidatorVerifier,
        validator_signer: Arc<ValidatorSigner>,
        batch_expiry_round_gap_when_init: Round,
        batch_expiry_round_gap_behind_latest_certified: Round,
        batch_expiry_round_gap_beyond_latest_certified: Round,
        batch_expiry_grace_rounds: Round,
        batch_request_num_peers: usize,
        batch_request_timeout_ms: usize,
        memory_quota: usize,
        db_quota: usize,
    ) -> (Self, Arc<BatchReader>) {
        let db_content = db.get_all_batches().expect("failed to read data from db");
        debug!("QS: db size {}", db_content.len());

        let (batch_reader, expired_keys) = BatchReader::new(
            epoch,
            last_committed_round,
            db_content,
            my_peer_id,
            batch_store_tx,
            batch_reader_tx,
            batch_expiry_round_gap_when_init,
            batch_expiry_round_gap_behind_latest_certified,
            batch_expiry_round_gap_beyond_latest_certified,
            batch_expiry_grace_rounds,
            memory_quota,
            db_quota,
        );
        if let Err(e) = db.delete_batches(expired_keys) {
            debug!("Error deleting batches: {:?}", e)
        }
        let batch_reader_clone = batch_reader.clone();
        let net = network_sender.clone();
        let metrics_monitor = tokio_metrics::TaskMonitor::new();

        tokio::spawn(async move {
            metrics_monitor
                .instrument(batch_reader_clone.start(
                    batch_reader_rx,
                    net,
                    batch_request_num_peers,
                    batch_request_timeout_ms,
                    validator_verifier,
                ))
                .await
        });

        let batch_reader_clone = batch_reader.clone();
        (
            Self {
                epoch,
                my_peer_id,
                network_sender,
                batch_reader,
                db,
                validator_signer,
            },
            batch_reader_clone,
        )
    }

    fn store(&self, persist_request: PersistRequest) -> Option<SignedDigest> {
        debug!("QS: store");
        let expiration = persist_request.value.expiration;
        // Network listener should filter messages with wrong expiration epoch.
        assert_eq!(
            expiration.epoch(),
            self.epoch,
            "Persist Request for a batch with an incorrect epoch"
        );
        match self
            .batch_reader
            .save(persist_request.digest, persist_request.value.clone()) // TODO: what is this comes from old epoch?
        {
            Ok(needs_db) => {
                let batch_author = persist_request.value.author;
                let num_txns = persist_request.value.maybe_payload.as_ref().unwrap().len() as u64;
                let num_bytes = persist_request.value.num_bytes as u64;
                debug!("QS: sign digest");
                if needs_db {
                    // TODO: Consider an async call to DB, but it could be a race with clean.
                    self.db
                        .save_batch(persist_request.digest, persist_request.value)
                        .expect("Could not write to DB");
                }
                Some(SignedDigest::new(
                    batch_author,
                    self.epoch,
                    persist_request.digest,
                    expiration,
                    num_txns,
                    num_bytes,
                    self.validator_signer.clone(),
                ).unwrap())
            }

            Err(e) => {
                debug!("QS: failed to store to cache {:?}", e);
                None
            }
        }
    }

    pub async fn start(
        self,
        mut batch_store_rx: Receiver<BatchStoreCommand>,
        proof_coordinator_tx: Sender<ProofCoordinatorCommand>,
    ) {
        debug!(
            "[QS worker] BatchStore worker for epoch {} starting",
            self.epoch
        );

        while let Some(command) = batch_store_rx.recv().await {
            match command {
                BatchStoreCommand::Shutdown(ack_tx) => {
                    self.batch_reader.shutdown().await;
                    ack_tx
                        .send(())
                        .expect("Failed to send shutdown ack to QuorumStore");
                    break;
                },
                BatchStoreCommand::Persist(persist_request) => {
                    let author = persist_request.value.author;
                    if let Some(signed_digest) = self.store(persist_request) {
                        if self.my_peer_id == author {
                            proof_coordinator_tx
                                .send(ProofCoordinatorCommand::AppendSignature(signed_digest))
                                .await
                                .expect("Failed to send to ProofBuilder");
                            debug!("QS: sent signed digest to ProofBuilder");
                        } else {
                            // Quorum store metrics
                            counters::DELIVERED_BATCHES_COUNT.inc();

                            self.network_sender
                                .send_signed_digest(signed_digest, vec![author])
                                .await;
                            debug!("QS: sent signed digest back to sender");
                        }
                    }
                },
                BatchStoreCommand::Clean(digests) => {
                    if let Err(e) = self.db.delete_batches(digests) {
                        debug!("Error deleting batches: {:?}", e)
                    }
                },
                BatchStoreCommand::BatchRequest(digest, peer_id, maybe_tx) => {
                    counters::GET_BATCH_FROM_DB_COUNT.inc();

                    match self.db.get_batch(digest) {
                        Ok(Some(persisted_value)) => {
                            let payload = persisted_value
                                .maybe_payload
                                .expect("Persisted value in QuorumStore DB must have payload");
                            match maybe_tx {
                                Some(payload_tx) => {
                                    assert_eq!(
                                        self.my_peer_id, peer_id,
                                        "Return channel must be to self"
                                    );
                                    if payload_tx.send(Ok(payload)).is_err() {
                                        debug!(
                                            "Failed to send PersistedValue for digest {}",
                                            digest
                                        );
                                    }
                                },
                                None => {
                                    assert_ne!(
                                        self.my_peer_id, peer_id,
                                        "Request from self without return channel"
                                    );
                                    let batch =
                                        Batch::new(self.my_peer_id, self.epoch, digest, payload);
                                    self.network_sender.send_batch(batch, vec![peer_id]).await;
                                },
                            }
                        },
                        Ok(None) => unreachable!(
                            "Could not read persisted value (according to BatchReader) from DB"
                        ),
                        Err(_) => {
                            // TODO: handle error, e.g. from self or not, log, panic.
                        },
                    }
                },
            }
        }

        debug!(
            "[QS worker] BatchStore worker for epoch {} stopping",
            self.epoch
        );
    }
}
