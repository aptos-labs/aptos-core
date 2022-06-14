// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network_interface::ConsensusMsg;
use crate::quorum_store::types::{Batch, Data, PersistedValue};
use crate::{
    network::NetworkSender,
    quorum_store::{
        batch_reader::{BatchReader, BatchReaderCommand},
        quorum_store_db::QuorumStoreDB,
    },
};
use aptos_crypto::HashValue;
use aptos_types::{
    validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier, PeerId,
};
use consensus_types::common::Round;
use consensus_types::proof_of_store::{LogicalTime, SignedDigest};
use serde::{Deserialize, Serialize};
use std::sync::{
    mpsc::{Receiver as SyncReceiver, SyncSender},
    Arc,
};
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
        payload: Data,
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
    Persist(PersistRequest, Option<oneshot::Sender<SignedDigest>>),
    BatchRequest(HashValue, PeerId, Option<oneshot::Sender<Data>>),
    Clean(Vec<HashValue>),
}

///gets PersistRequest, persist, sign and return (network or oneshot to self)
pub(crate) struct BatchStore {
    epoch: u64,
    my_peer_id: PeerId,
    network_sender: NetworkSender,
    batch_reader: Arc<BatchReader>,
    db: Arc<QuorumStoreDB>,
    validator_signer: Arc<ValidatorSigner>,
}

impl BatchStore {
    pub fn new(
        epoch: u64,
        last_committed_round: Round,
        my_peer_id: PeerId,
        network_sender: NetworkSender,
        batch_store_tx: Sender<BatchStoreCommand>,
        batch_reader_tx: SyncSender<BatchReaderCommand>,
        batch_reader_rx: SyncReceiver<BatchReaderCommand>,
        db: Arc<QuorumStoreDB>,
        validator_signer: Arc<ValidatorSigner>,
        max_execution_round_lag: Round,
        batch_request_num_peers: usize,
        batch_request_timeout_ms: usize,
        memory_quota: usize,
        db_quota: usize,
    ) -> (Self, Arc<BatchReader>) {
        let db_content = db.get_data().expect("failed to read data from db");

        let (batch_reader, expired_keys) = BatchReader::new(
            epoch,
            last_committed_round,
            db_content,
            my_peer_id,
            batch_store_tx,
            batch_reader_tx,
            max_execution_round_lag,
            memory_quota,
            db_quota,
        );
        if let Err(_) = db.delete(expired_keys) {
            // TODO: do something
        }
        let batch_reader: Arc<BatchReader> = Arc::new(batch_reader);
        let batch_reader_clone = batch_reader.clone();
        let validator_signer_clone = validator_signer.clone();
        let net = network_sender.clone();
        tokio::spawn(async move {
            batch_reader_clone
                .start(
                    batch_reader_rx,
                    net,
                    validator_signer_clone,
                    batch_request_num_peers,
                    batch_request_timeout_ms,
                )
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
        let expiration = persist_request.value.expiration.clone();
        // Network listener should filter messages with wrong expiration epoch.
        assert_eq!(
            expiration.epoch(),
            self.epoch,
            "Persist Request for a batch with an incorrect epoch"
        );

        if self
            .batch_reader
            .save(persist_request.digest, persist_request.value.clone())
        {
            //TODO: Consider an async call to DB, but it could be a race with clean.
            self.db
                .save_batch(persist_request.digest, persist_request.value)
                .expect("Could not write to DB");
            Some(SignedDigest::new(
                self.epoch,
                self.my_peer_id,
                persist_request.digest,
                expiration,
                self.validator_signer.clone(),
            ))
        } else {
            // Request to store a batch for longer than maximum gap.
            None
        }
    }

    pub async fn start(
        self,
        _validator_verifier: ValidatorVerifier,
        mut batch_store_rx: Receiver<BatchStoreCommand>,
    ) {
        while let Some(command) = batch_store_rx.recv().await {
            match command {
                BatchStoreCommand::Persist(persist_request, maybe_tx) => {
                    let author = persist_request.value.author;
                    if let Some(signed_digest) = self.store(persist_request) {
                        if let Some(ack_tx) = maybe_tx {
                            debug_assert!(
                                self.my_peer_id == author,
                                "Persist request with return channel must be from self"
                            );
                            ack_tx
                                .send(signed_digest)
                                .expect("Failed to send signed digest");
                        } else {
                            let msg = ConsensusMsg::SignedDigestMsg(Box::new(signed_digest));
                            self.network_sender.send(msg, vec![author]).await;
                        }
                    }
                }
                BatchStoreCommand::Clean(digests) => {
                    if let Err(_) = self.db.delete(digests) {
                        //TODO: do something
                    }
                }
                BatchStoreCommand::BatchRequest(digest, peer_id, maybe_tx) => {
                    match self.db.get_batch(digest) {
                        Ok(maybe_persisted_value) => {
                            if self.my_peer_id == peer_id {
                                //ok to unwrap because value is guaranteed to be in the db and have actual payload
                                maybe_tx
                                    .unwrap()
                                    .send(maybe_persisted_value.unwrap().maybe_payload.unwrap())
                                    .expect("Failed to send PersistedValue");
                            } else {
                                let batch = Batch::new(
                                    self.epoch,
                                    self.my_peer_id,
                                    digest,
                                    Some(maybe_persisted_value.unwrap().maybe_payload.unwrap()),
                                    self.validator_signer.clone(),
                                );
                                let msg = ConsensusMsg::BatchMsg(Box::new(batch));
                                self.network_sender.send(msg, vec![peer_id]).await;
                            }
                        }
                        Err(_) => {
                            //TODO: do something
                        }
                    }
                }
            }
        }
    }
}
