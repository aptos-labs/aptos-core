// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    quorum_store::{
        batch_requester::BatchRequester, counters, quorum_store_db::QuorumStoreStorage,
        types::PersistedValue, utils::RoundExpirations,
    },
};
use anyhow::bail;
use aptos_consensus_types::{
    common::Round,
    proof_of_store::{LogicalTime, ProofOfStore, SignedDigest},
};
use aptos_crypto::HashValue;
use aptos_executor_types::Error;
use aptos_logger::prelude::*;
use aptos_types::{
    transaction::SignedTransaction, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier, PeerId,
};
use dashmap::{
    mapref::entry::Entry::{Occupied, Vacant},
    DashMap,
};
use fail::fail_point;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio::sync::oneshot;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PersistRequest {
    pub digest: HashValue,
    pub value: PersistedValue,
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
pub struct BatchStore<T> {
    epoch: OnceCell<u64>,
    last_certified_round: AtomicU64,
    db_cache: DashMap<HashValue, PersistedValue>,
    peer_quota: DashMap<PeerId, QuotaManager>,
    expirations: Mutex<RoundExpirations<HashValue>>,
    db: Arc<dyn QuorumStoreStorage>,
    batch_expiry_round_gap_when_init: Round,
    batch_expiry_round_gap_behind_latest_certified: Round,
    batch_expiry_round_gap_beyond_latest_certified: Round,
    expiry_grace_rounds: Round,
    memory_quota: usize,
    db_quota: usize,
    batch_requester: BatchRequester<T>,
    validator_signer: ValidatorSigner,
    validator_verifier: ValidatorVerifier,
}

impl<T: QuorumStoreSender + Clone + Send + Sync + 'static> BatchStore<T> {
    pub(crate) fn new(
        epoch: u64,
        last_certified_round: Round,
        db: Arc<dyn QuorumStoreStorage>,
        batch_expiry_round_gap_when_init: Round,
        batch_expiry_round_gap_behind_latest_certified: Round,
        batch_expiry_round_gap_beyond_latest_certified: Round,
        expiry_grace_rounds: Round,
        memory_quota: usize,
        db_quota: usize,
        batch_requester: BatchRequester<T>,
        validator_signer: ValidatorSigner,
        validator_verifier: ValidatorVerifier,
    ) -> Self {
        let db_clone = db.clone();
        let batch_store = Self {
            epoch: OnceCell::with_value(epoch),
            last_certified_round: AtomicU64::new(last_certified_round),
            db_cache: DashMap::new(),
            peer_quota: DashMap::new(),
            expirations: Mutex::new(RoundExpirations::new()),
            db,
            batch_expiry_round_gap_when_init,
            batch_expiry_round_gap_behind_latest_certified,
            batch_expiry_round_gap_beyond_latest_certified,
            expiry_grace_rounds,
            memory_quota,
            db_quota,
            batch_requester,
            validator_signer,
            validator_verifier,
        };
        let db_content = db_clone
            .get_all_batches()
            .expect("failed to read data from db");
        let mut expired_keys = Vec::new();
        trace!(
            "QS: Batchreader {} {} {}",
            db_content.len(),
            epoch,
            last_certified_round
        );
        for (digest, value) in db_content {
            let expiration = value.expiration;

            trace!(
                "QS: Batchreader recovery content exp {:?}, digest {}",
                expiration,
                digest
            );
            assert!(epoch >= expiration.epoch());

            if epoch > expiration.epoch()
                || last_certified_round >= expiration.round() + expiry_grace_rounds
            {
                expired_keys.push(digest);
            } else {
                batch_store
                    .update_cache(digest, value)
                    .expect("Storage limit exceeded upon BatchReader construction");
            }
        }
        trace!(
            "QS: Batchreader recovery expired keys len {}",
            expired_keys.len()
        );
        db_clone.delete_batches(expired_keys).unwrap();

        batch_store
    }

    fn epoch(&self) -> u64 {
        *self.epoch.get().unwrap()
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
                        debug!(
                            "QS: already have the digest with higher expiration {}",
                            digest
                        );
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

    pub fn persist(&self, persist_request: PersistRequest) -> Option<SignedDigest> {
        let expiration = persist_request.value.expiration;
        // Network listener should filter messages with wrong expiration epoch.
        assert_eq!(
            expiration.epoch(),
            self.epoch(),
            "Persist Request for a batch with an incorrect epoch"
        );

        match self.save(persist_request.digest, persist_request.value.clone()) {
            Ok(needs_db) => {
                let num_txns = persist_request.value.maybe_payload.as_ref().unwrap().len() as u64;
                let num_bytes = persist_request.value.num_bytes as u64;
                let batch_author = persist_request.value.author;
                trace!("QS: sign digest {}", persist_request.digest);
                if needs_db {
                    self.db
                        .save_batch(persist_request.digest, persist_request.value)
                        .expect("Could not write to DB");
                }
                Some(
                    SignedDigest::new(
                        batch_author,
                        self.epoch(),
                        persist_request.digest,
                        expiration,
                        num_txns,
                        num_bytes,
                        &self.validator_signer,
                    )
                    .unwrap(),
                )
            },

            Err(e) => {
                debug!("QS: failed to store to cache {:?}", e);
                None
            },
        }
    }

    // TODO: make sure state-sync also sends the message, or execution cleans.
    // When self.expiry_grace_rounds == 0, certified time contains a round for
    // which execution result has been certified by a quorum, and as such, the
    // batches with expiration in this round can be cleaned up. The parameter
    // expiry grace rounds just keeps the batches around for a little longer
    // for lagging nodes to be able to catch up (without state-sync).
    pub async fn update_certified_round(&self, certified_time: LogicalTime) {
        trace!("QS: batch reader updating time {:?}", certified_time);
        assert_eq!(
            self.epoch(),
            certified_time.epoch(),
            "QS: wrong epoch {} != {}",
            self.epoch(),
            certified_time.epoch()
        );

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
        if let Err(e) = self.db.delete_batches(expired_keys) {
            debug!("Error deleting batches: {:?}", e)
        }
    }

    fn last_certified_round(&self) -> Round {
        self.last_certified_round.load(Ordering::Relaxed)
    }

    fn get_batch_from_db(&self, digest: &HashValue) -> Result<Vec<SignedTransaction>, Error> {
        counters::GET_BATCH_FROM_DB_COUNT.inc();

        match self.db.get_batch(digest) {
            Ok(Some(persisted_value)) => {
                let payload = persisted_value
                    .maybe_payload
                    .expect("Persisted value in QuorumStore DB must have payload");
                return Ok(payload);
            },
            Ok(None) => {
                unreachable!("Could not read persisted value (according to BatchReader) from DB")
            },
            Err(_) => {
                // TODO: handle error, e.g. from self or not, log, panic.
            },
        }
        Err(Error::CouldNotGetData)
    }

    pub fn get_batch_from_local(
        &self,
        digest: &HashValue,
    ) -> Result<Vec<SignedTransaction>, Error> {
        if let Some(value) = self.db_cache.get(digest) {
            if payload_storage_mode(&value) == StorageMode::PersistedOnly {
                assert!(
                    value.maybe_payload.is_none(),
                    "BatchReader payload and storage kind mismatch"
                );
                self.get_batch_from_db(digest)
            } else {
                // Available in memory.
                Ok(value
                    .maybe_payload
                    .clone()
                    .expect("BatchReader payload and storage kind mismatch"))
            }
        } else {
            Err(Error::CouldNotGetData)
        }
    }

    pub fn get_batch(
        &self,
        proof: ProofOfStore,
    ) -> oneshot::Receiver<Result<Vec<SignedTransaction>, Error>> {
        let (tx, rx) = oneshot::channel();

        if let Ok(value) = self.get_batch_from_local(proof.digest()) {
            tx.send(Ok(value)).unwrap();
        } else {
            // Quorum store metrics
            counters::MISSED_BATCHES_COUNT.inc();
            self.batch_requester.request_batch(
                *proof.digest(),
                proof.shuffled_signers(&self.validator_verifier),
                tx,
            );
        }
        rx
    }
}
