// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network::QuorumStoreSender;
use crate::quorum_store::{batch_requester::BatchRequester, batch_store::BatchStoreCommand};
use crate::quorum_store::{
    counters,
    types::{Batch, PersistedValue},
    utils::RoundExpirations,
};
use anyhow::bail;
use aptos_crypto::HashValue;
use aptos_logger::debug;
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_types::{transaction::SignedTransaction, PeerId};
use consensus_types::{
    common::Round,
    proof_of_store::{LogicalTime, ProofOfStore},
};
use dashmap::DashMap;
use executor_types::Error;
use once_cell::sync::OnceCell;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    time::Duration,
};
use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        oneshot, Notify,
    },
    time,
};

#[derive(Debug)]
pub(crate) enum BatchReaderCommand {
    GetBatchForPeer(HashValue, PeerId),
    GetBatchForSelf(
        ProofOfStore,
        oneshot::Sender<Result<Vec<SignedTransaction>, Error>>,
    ),
    BatchResponse(HashValue, Vec<SignedTransaction>),
}

#[derive(PartialEq)]
enum StorageMode {
    PersistedOnly,
    MemoryAndPersisted,
}

struct QuotaManager {
    memory_balance: usize,
    db_balance: usize,
    memory_quota: usize,
    db_quota: usize,
}

impl QuotaManager {
    fn new(max_db_balance: usize, max_memory_balance: usize) -> Self {
        assert!(max_db_balance >= max_memory_balance);
        Self {
            memory_balance: 0,
            db_balance: 0,
            memory_quota: max_memory_balance,
            db_quota: max_db_balance,
        }
    }

    pub(crate) fn update_quota(&mut self, num_bytes: usize) -> anyhow::Result<StorageMode> {
        if self.memory_balance + num_bytes <= self.memory_quota {
            self.memory_balance = self.memory_balance + num_bytes;
            self.db_balance = self.db_balance + num_bytes;
            Ok(StorageMode::MemoryAndPersisted)
        } else if self.db_balance + num_bytes <= self.db_quota {
            self.db_balance = self.db_balance + num_bytes;
            Ok(StorageMode::PersistedOnly)
        } else {
            counters::EXCEEDED_STORAGE_QUOTA_COUNT.inc();
            bail!("Storage quota exceeded ");
        }
    }

    pub(crate) fn free_quota(&mut self, num_bytes: usize, storage_mode: StorageMode) {
        match storage_mode {
            StorageMode::PersistedOnly => {
                self.db_balance = self.db_balance - num_bytes;
            }
            StorageMode::MemoryAndPersisted => {
                self.memory_balance = self.memory_balance - num_bytes;
                self.db_balance = self.db_balance - num_bytes;
            }
        }
    }
}

fn payload_storage_mode(persisted_value: &PersistedValue) -> StorageMode {
    match persisted_value.maybe_payload {
        Some(_) => StorageMode::MemoryAndPersisted,
        None => StorageMode::PersistedOnly,
    }
}

/// Provides in memory representation of stored batches (strong cache), and allows
/// efficient concurrent readers.
pub struct BatchReader {
    epoch: OnceCell<u64>,
    my_peer_id: PeerId,
    last_committed_round: AtomicU64,
    db_cache: DashMap<HashValue, PersistedValue>,
    peer_quota: DashMap<PeerId, QuotaManager>,
    expirations: Mutex<RoundExpirations<HashValue>>,
    batch_store_tx: Sender<BatchStoreCommand>,
    self_tx: Sender<BatchReaderCommand>,
    max_expiry_round_gap: Round,
    expiry_grace_rounds: Round,
    memory_quota: usize,
    db_quota: usize,
    shutdown_flag: AtomicBool,
    shutdown_notify: Notify,
}

impl BatchReader {
    pub(crate) fn new(
        epoch: u64,
        last_committed_round: Round,
        db_content: HashMap<HashValue, PersistedValue>,
        my_peer_id: PeerId,
        batch_store_tx: Sender<BatchStoreCommand>,
        self_tx: Sender<BatchReaderCommand>,
        max_expiry_round_gap: Round,
        expiry_grace_rounds: Round,
        memory_quota: usize,
        db_quota: usize,
    ) -> (Self, Vec<HashValue>) {
        let self_ob = Self {
            epoch: OnceCell::with_value(epoch),
            my_peer_id,
            last_committed_round: AtomicU64::new(last_committed_round),
            db_cache: DashMap::new(),
            peer_quota: DashMap::new(),
            expirations: Mutex::new(RoundExpirations::new()),
            batch_store_tx,
            self_tx,
            max_expiry_round_gap,
            expiry_grace_rounds,
            memory_quota,
            db_quota,
            shutdown_flag: AtomicBool::new(false),
            shutdown_notify: Notify::new(),
        };

        let mut expired_keys = Vec::new();
        debug!(
            "QS: Batchreader {} {} {}",
            db_content.len(),
            epoch,
            last_committed_round
        );
        for (digest, value) in db_content {
            let expiration = value.expiration;

            debug!(
                "QS: Batchreader recovery content exp {:?}, digest {}",
                expiration, digest
            );
            assert!(epoch >= expiration.epoch());

            if epoch > expiration.epoch()
                || last_committed_round > expiration.round() + expiry_grace_rounds
            {
                expired_keys.push(digest);
            } else {
                self_ob
                    .update_cache(digest, value)
                    .expect("Storage limit exceeded upon BatchReader construction");
            }
        }

        debug!(
            "QS: Batchreader recovery expired keys len {}",
            expired_keys.len()
        );
        (self_ob, expired_keys)
    }

    fn epoch(&self) -> u64 {
        *self.epoch.get().unwrap()
    }

    // Return an error if storage quota is exceeded.
    fn update_cache(&self, digest: HashValue, mut value: PersistedValue) -> anyhow::Result<()> {
        let author = value.author;
        if self
            .peer_quota
            .entry(author)
            .or_insert(QuotaManager::new(self.db_quota, self.memory_quota))
            .update_quota(value.num_bytes)?
            == StorageMode::PersistedOnly
        {
            value.remove_payload();
        }

        let expiration_round = value.expiration.round();
        if let Some(prev_value) = self.db_cache.insert(digest, value) {
            self.free_quota(prev_value);
        }
        self.expirations
            .lock()
            .unwrap()
            .add_item(digest, expiration_round);
        Ok(())
    }

    pub(crate) fn save(&self, digest: HashValue, value: PersistedValue) -> anyhow::Result<bool> {
        if value.expiration.epoch() == self.epoch()
            && value.expiration.round() > self.last_committed_round()
            && value.expiration.round() <= self.last_committed_round() + self.max_expiry_round_gap
        {
            if let Some(entry) = self.db_cache.get(&digest) {
                if entry.expiration.round() >= value.expiration.round() {
                    debug!("QS: already have the digest with higher expiration");
                    return Ok(false);
                }
            }
            self.update_cache(digest, value)?;
            Ok(true)
        } else {
            bail!("Incorrect expiration {:?} for BatchReader in epoch {}, last committed round {} and max gap {}",
	      value.expiration,
	      self.epoch(),
	      self.last_committed_round(),
	      self.max_expiry_round_gap);
        }
    }

    pub async fn shutdown(&self) {
        // if !self.shutdown_flag.swap(true, Ordering::Relaxed) {
        self.shutdown_flag.swap(true, Ordering::Relaxed);
        self.shutdown_notify.notified().await;
    }

    fn clear_expired_payload(&self, certified_time: LogicalTime) -> Vec<HashValue> {
        assert_eq!(
            certified_time.epoch(),
            self.epoch(),
            "Execution epoch inconsistent with BatchReader"
        );

        let expired_round = if certified_time.round() >= self.expiry_grace_rounds {
            certified_time.round() - self.expiry_grace_rounds
        } else {
            0
        };

        let expired_digests = self.expirations.lock().unwrap().expire(expired_round);
        let mut ret = Vec::new();
        for h in expired_digests {
            let cache_expiration_round = self
                .db_cache
                .get(&h)
                .expect("Expired entry not in cache")
                .expiration
                .round();

            // We need to check up-to-date expiration again because receiving the same
            // digest with a higher expiration would update the persisted value and
            // effectively extend the expiration.
            if cache_expiration_round < expired_round {
                let (_, persisted_value) = self
                    .db_cache
                    .remove(&h)
                    .expect("Expired entry not in cache");
                self.free_quota(persisted_value);
                ret.push(h);
            } // Otherwise, expiration got extended, ignore.
        }
        ret
    }

    fn free_quota(&self, persisted_value: PersistedValue) {
        let mut quota_manager = self
            .peer_quota
            .get_mut(&persisted_value.author)
            .expect("No QuotaManager for batch author");
        quota_manager.free_quota(
            persisted_value.num_bytes,
            payload_storage_mode(&persisted_value),
        );
    }

    // TODO: make sure state-sync also sends the message, or execution cleans.
    pub async fn update_certified_round(&self, certified_time: LogicalTime) {
        debug!("QS: batch reader updating time {:?}", certified_time);
        let prev_round = self
            .last_committed_round
            .fetch_max(certified_time.round(), Ordering::SeqCst);
        assert!(
            prev_round < certified_time.round(),
            "Non-increasing executed rounds reported to BatchStore"
        );
        let expired_keys = self.clear_expired_payload(certified_time);
        if let Err(e) = self
            .batch_store_tx
            .send(BatchStoreCommand::Clean(expired_keys))
            .await
        {
            debug!("QS: Failed to send to BatchStore: {:?}", e);
        }
    }

    fn last_committed_round(&self) -> Round {
        self.last_committed_round.load(Ordering::Relaxed)
    }

    pub async fn get_batch(
        &self,
        proof: ProofOfStore,
    ) -> oneshot::Receiver<Result<Vec<SignedTransaction>, Error>> {
        let (tx, rx) = oneshot::channel();

        if let Some(value) = self.db_cache.get(&proof.digest()) {
            if payload_storage_mode(&value) == StorageMode::PersistedOnly {
                assert!(
                    value.maybe_payload.is_none(),
                    "BatchReader payload and storage kind mismatch"
                );
                self.batch_store_tx
                    .send(BatchStoreCommand::BatchRequest(
                        proof.digest().clone(),
                        self.my_peer_id,
                        Some(tx),
                    ))
                    .await
                    .expect("Failed to send to BatchStore");
            } else {
                // Available in memory.
                tx.send(Ok(value
                    .maybe_payload
                    .clone()
                    .expect("BatchReader payload and storage kind mismatch")))
                    .expect("Receiver of requested batch is not available");
            }
        } else {
            // Quorum store metrics
            counters::MISSED_BATCHES_COUNT.inc();

            self.self_tx
                .send(BatchReaderCommand::GetBatchForSelf(proof, tx))
                .await
                .expect("Batch Reader Receiver is not available");
        }
        rx
    }

    pub(crate) async fn start<T: QuorumStoreSender + Clone>(
        &self,
        mut batch_reader_rx: Receiver<BatchReaderCommand>,
        network_sender: T,
        request_num_peers: usize,
        request_timeout_ms: usize,
        verifier: ValidatorVerifier,
    ) {
        debug!(
            "[QS worker] BatchReader worker for epoch {} starting",
            self.epoch()
        );

        let mut batch_requester = BatchRequester::new(
            self.epoch(),
            self.my_peer_id,
            request_num_peers,
            request_timeout_ms,
            network_sender.clone(),
        );

        // TODO: experiment / decide on the parameter for generating the interval duration (i.e. "/2").
        let mut interval = time::interval(Duration::from_millis((request_timeout_ms / 2) as u64));

        loop {
            // TODO: shutdown?
            tokio::select! {
                biased;

                _ = interval.tick() => {
                    batch_requester.handle_timeouts().await;
            if self.shutdown_flag.load(Ordering::Relaxed) {
            break;
            }
                },

                Some(cmd) = batch_reader_rx.recv() => {
                    match cmd {
            BatchReaderCommand::GetBatchForPeer(digest, peer_id) => {
                if let Some(value) = self.db_cache.get(&digest) {

            match payload_storage_mode(&value) {
                StorageMode::PersistedOnly => {
                    assert!(value.maybe_payload.is_none(),
                                            "BatchReader payload and storage kind mismatch");
                    self.batch_store_tx
                    .send(BatchStoreCommand::BatchRequest(digest, peer_id, None))
                    .await
                    .expect("Failed to send to BatchStore"); // TODO: I think we have a race here. Batch store can stop before batch reader.
                },
            StorageMode::MemoryAndPersisted => {
                    let batch = Batch::new(
                    self.epoch(),
                    self.my_peer_id,
                    digest,
                    Some(value.maybe_payload.clone().expect("BatchReader payload and storage kind mismatch")),
                    );
                    network_sender.send_batch(batch, vec![peer_id]).await;
            }
                    } // TODO: consider returning Nack
                }
            }
                        BatchReaderCommand::GetBatchForSelf(proof, ret_tx) => {
                            batch_requester
                                .add_request(proof.digest().clone(), proof.shuffled_signers(&verifier), ret_tx)
                                .await;
                        }
                        BatchReaderCommand::BatchResponse(digest, payload) => {
                            batch_requester.serve_request(digest, payload);
                        }
                    }
                }
            }
        }

        self.shutdown_notify.notify_one();
        debug!(
            "[QS worker] BatchReader worker for epoch {} stopping",
            self.epoch()
        );
    }
}
