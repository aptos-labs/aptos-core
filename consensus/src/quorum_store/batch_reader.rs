// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    quorum_store::{
        batch_requester::BatchRequester,
        batch_store::BatchStoreCommand,
        counters,
        types::{Batch, PersistedValue},
        utils::RoundExpirations,
    },
};
use anyhow::bail;
use aptos_consensus_types::{
    common::Round,
    proof_of_store::{LogicalTime, ProofOfStore},
};
use aptos_crypto::HashValue;
use aptos_executor_types::Error;
use aptos_logger::debug;
use aptos_types::{transaction::SignedTransaction, validator_verifier::ValidatorVerifier, PeerId};
use dashmap::{
    mapref::entry::Entry::{Occupied, Vacant},
    DashMap,
};
use fail::fail_point;
use once_cell::sync::OnceCell;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
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
            self.memory_balance += num_bytes;
            self.db_balance += num_bytes;
            Ok(StorageMode::MemoryAndPersisted)
        } else if self.db_balance + num_bytes <= self.db_quota {
            self.db_balance += num_bytes;
            Ok(StorageMode::PersistedOnly)
        } else {
            counters::EXCEEDED_STORAGE_QUOTA_COUNT.inc();
            bail!("Storage quota exceeded ");
        }
    }

    pub(crate) fn free_quota(&mut self, num_bytes: usize, storage_mode: StorageMode) {
        match storage_mode {
            StorageMode::PersistedOnly => {
                self.db_balance -= num_bytes;
            },
            StorageMode::MemoryAndPersisted => {
                self.memory_balance -= num_bytes;
                self.db_balance -= num_bytes;
            },
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
    last_certified_round: AtomicU64,
    db_cache: DashMap<HashValue, PersistedValue>,
    peer_quota: DashMap<PeerId, QuotaManager>,
    expirations: Mutex<RoundExpirations<HashValue>>,
    batch_store_tx: Sender<BatchStoreCommand>,
    self_tx: Sender<BatchReaderCommand>,
    batch_expiry_round_gap_when_init: Round,
    batch_expiry_round_gap_behind_latest_certified: Round,
    batch_expiry_round_gap_beyond_latest_certified: Round,
    expiry_grace_rounds: Round,
    memory_quota: usize,
    db_quota: usize,
    shutdown_flag: AtomicBool,
    shutdown_notify: Notify,
}

impl BatchReader {
    pub(crate) fn new(
        epoch: u64,
        last_certified_round: Round,
        db_content: HashMap<HashValue, PersistedValue>,
        my_peer_id: PeerId,
        batch_store_tx: Sender<BatchStoreCommand>,
        self_tx: Sender<BatchReaderCommand>,
        batch_expiry_round_gap_when_init: Round,
        batch_expiry_round_gap_behind_latest_certified: Round,
        batch_expiry_round_gap_beyond_latest_certified: Round,
        expiry_grace_rounds: Round,
        memory_quota: usize,
        db_quota: usize,
    ) -> (Arc<Self>, Vec<HashValue>) {
        let self_ob = Self {
            epoch: OnceCell::with_value(epoch),
            my_peer_id,
            last_certified_round: AtomicU64::new(last_certified_round),
            db_cache: DashMap::new(),
            peer_quota: DashMap::new(),
            expirations: Mutex::new(RoundExpirations::new()),
            batch_store_tx,
            self_tx,
            batch_expiry_round_gap_when_init,
            batch_expiry_round_gap_behind_latest_certified,
            batch_expiry_round_gap_beyond_latest_certified,
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
            last_certified_round
        );
        for (digest, value) in db_content {
            let expiration = value.expiration;

            debug!(
                "QS: Batchreader recovery content exp {:?}, digest {}",
                expiration, digest
            );
            assert!(epoch >= expiration.epoch());

            if epoch > expiration.epoch()
                || last_certified_round >= expiration.round() + expiry_grace_rounds
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
        (Arc::new(self_ob), expired_keys)
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
        if value.expiration.epoch() == self.epoch() {
            // record the round gaps
            if value.expiration.round() > self.last_certified_round() {
                counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_LAST_CERTIFIED_ROUND_HIGHER
                    .observe((value.expiration.round() - self.last_certified_round()) as f64);
            }
            if value.expiration.round() < self.last_certified_round() {
                counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_LAST_CERTIFIED_ROUND_LOWER
                    .observe((self.last_certified_round() - value.expiration.round()) as f64);
            }

            if value.expiration.round() + self.batch_expiry_round_gap_behind_latest_certified
                >= self.last_certified_round()
                && value.expiration.round()
                    <= self.last_certified_round()
                        + self.batch_expiry_round_gap_beyond_latest_certified
            {
                fail_point!("quorum_store::save", |_| {
                    // Skip caching and storing value to the db
                    Ok(false)
                });

                if let Some(entry) = self.db_cache.get(&digest) {
                    if entry.expiration.round() >= value.expiration.round() {
                        debug!("QS: already have the digest with higher expiration");
                        return Ok(false);
                    }
                }
                self.update_cache(digest, value)?;
                return Ok(true);
            }
        }
        bail!("Incorrect expiration {:?} with init gap {} in epoch {}, last committed round {} and max behind gap {} max beyond gap {}",
            value.expiration,
            self.batch_expiry_round_gap_when_init,
            self.epoch(),
            self.last_certified_round(),
            self.batch_expiry_round_gap_behind_latest_certified,
            self.batch_expiry_round_gap_beyond_latest_certified);
    }

    pub async fn shutdown(&self) {
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
            let removed_value = match self.db_cache.entry(h) {
                Occupied(entry) => {
                    // We need to check up-to-date expiration again because receiving the same
                    // digest with a higher expiration would update the persisted value and
                    // effectively extend the expiration.
                    if entry.get().expiration.round() <= expired_round {
                        Some(entry.remove())
                    } else {
                        None
                    }
                },
                Vacant(_) => unreachable!("Expired entry not in cache"),
            };
            // No longer holding the lock on db_cache entry.
            if let Some(value) = removed_value {
                self.free_quota(value);
                ret.push(h);
            }
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
    // When self.expiry_grace_rounds == 0, certified time contains a round for
    // which execution result has been certified by a quorum, and as such, the
    // batches with expiration in this round can be cleaned up. The parameter
    // expiry grace rounds just keeps the batches around for a little longer
    // for lagging nodes to be able to catch up (without state-sync).
    pub async fn update_certified_round(&self, certified_time: LogicalTime) {
        debug!("QS: batch reader updating time {:?}", certified_time);
        assert!(self.epoch() == certified_time.epoch(), "QS: wrong epoch");

        let prev_round = self
            .last_certified_round
            .fetch_max(certified_time.round(), Ordering::SeqCst);
        // Note: prev_round may be equal to certified_time round due to state-sync
        // at the epoch boundary.
        assert!(
            prev_round <= certified_time.round(),
            "Decreasing executed rounds reported to BatchReader {} {}",
            prev_round,
            certified_time.round(),
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

    fn last_certified_round(&self) -> Round {
        self.last_certified_round.load(Ordering::Relaxed)
    }

    pub async fn get_batch(
        &self,
        proof: ProofOfStore,
    ) -> oneshot::Receiver<Result<Vec<SignedTransaction>, Error>> {
        let (tx, rx) = oneshot::channel();

        if let Some(value) = self.db_cache.get(proof.digest()) {
            if payload_storage_mode(&value) == StorageMode::PersistedOnly {
                assert!(
                    value.maybe_payload.is_none(),
                    "BatchReader payload and storage kind mismatch"
                );
                self.batch_store_tx
                    .send(BatchStoreCommand::BatchRequest(
                        *proof.digest(),
                        self.my_peer_id,
                        Some(tx),
                    ))
                    .await
                    .expect("Failed to send to BatchStore");
            } else {
                // Available in memory.
                if tx
                    .send(Ok(value
                        .maybe_payload
                        .clone()
                        .expect("BatchReader payload and storage kind mismatch")))
                    .is_err()
                {
                    debug!(
                        "Receiver of requested batch is not available for digest {}",
                        proof.digest()
                    );
                }
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

        let mut interval = time::interval(Duration::from_millis(100));

        loop {
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
                                        assert!(value.maybe_payload.is_none(), "BatchReader payload and storage kind mismatch");
                                        if self.batch_store_tx.send(BatchStoreCommand::BatchRequest(digest, peer_id, None)).await.is_err() {
                                            debug!("Failed to send request to BatchStore");
                                        }
                                    },
                                    StorageMode::MemoryAndPersisted => {
                                        let batch = Batch::new(
                                            self.my_peer_id,
                                            self.epoch(),
                                            digest,
                                            value.maybe_payload.clone().expect("BatchReader payload and storage kind mismatch"),
                                        );
                                        network_sender.send_batch(batch, vec![peer_id]).await;
                                    },
                                } // TODO: consider returning Nack
                            }
                        },
                        BatchReaderCommand::GetBatchForSelf(proof, ret_tx) => {
                            batch_requester
                                .add_request(*proof.digest(), proof.shuffled_signers(&verifier), ret_tx)
                                .await;
                        },
                        BatchReaderCommand::BatchResponse(digest, payload) => {
                            batch_requester.serve_request(digest, payload);
                        },
                    }
                },
            }
        }

        self.shutdown_notify.notify_one();
        debug!(
            "[QS worker] BatchReader worker for epoch {} stopping",
            self.epoch()
        );
    }
}
