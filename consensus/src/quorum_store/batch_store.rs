// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::QuorumStoreSender,
    quorum_store::{
        batch_requester::BatchRequester,
        counters,
        quorum_store_db::QuorumStoreStorage,
        types::{PersistedValue, StorageMode},
        utils::TimeExpirations,
    },
};
use anyhow::bail;
use aptos_consensus_types::proof_of_store::{BatchInfo, SignedBatchInfo};
use aptos_crypto::{CryptoMaterialError, HashValue};
use aptos_executor_types::{ExecutorError, ExecutorResult};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{transaction::SignedTransaction, validator_signer::ValidatorSigner, PeerId};
use dashmap::{
    mapref::entry::Entry::{Occupied, Vacant},
    DashMap,
};
use fail::fail_point;
use futures::{future::Shared, FutureExt};
use once_cell::sync::OnceCell;
use std::{
    collections::{BTreeSet, HashMap},
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::oneshot;

// Pub(crate) for testing only.
pub(crate) struct QuotaManager {
    memory_balance: usize,
    db_balance: usize,
    batch_balance: usize,
    // Recording the provided quotas for asserts.
    memory_quota: usize,
    db_quota: usize,
    batch_quota: usize,
}

impl QuotaManager {
    pub(crate) fn new(db_quota: usize, memory_quota: usize, batch_quota: usize) -> Self {
        assert!(db_quota >= memory_quota);
        Self {
            memory_balance: memory_quota,
            db_balance: db_quota,
            batch_balance: batch_quota,
            memory_quota,
            db_quota,
            batch_quota,
        }
    }

    pub(crate) fn update_quota(&mut self, num_bytes: usize) -> anyhow::Result<StorageMode> {
        if self.batch_balance == 0 {
            counters::EXCEEDED_BATCH_QUOTA_COUNT.inc();
            bail!("Batch quota exceeded ");
        }

        if self.db_balance >= num_bytes {
            self.batch_balance -= 1;
            self.db_balance -= num_bytes;

            if self.memory_balance >= num_bytes {
                self.memory_balance -= num_bytes;
                Ok(StorageMode::MemoryAndPersisted)
            } else {
                Ok(StorageMode::PersistedOnly)
            }
        } else {
            counters::EXCEEDED_STORAGE_QUOTA_COUNT.inc();
            bail!("Storage quota exceeded ");
        }
    }

    fn assert_quota(balance: usize, to_free: usize, quota: usize, kind: &str) {
        assert!(
            balance + to_free <= quota,
            "Balance {} + to_free {} more than {} quota {}",
            balance,
            to_free,
            kind,
            quota,
        );
    }

    pub(crate) fn free_quota(&mut self, num_bytes: usize, storage_mode: StorageMode) {
        Self::assert_quota(self.batch_balance, 1, self.batch_quota, "Batch");
        self.batch_balance += 1;

        Self::assert_quota(self.db_balance, num_bytes, self.db_quota, "DB");
        self.db_balance += num_bytes;

        if matches!(storage_mode, StorageMode::MemoryAndPersisted) {
            Self::assert_quota(self.memory_balance, num_bytes, self.memory_quota, "Memory");
            self.memory_balance += num_bytes;
        }
    }
}

/// Provides in memory representation of stored batches (strong cache), and allows
/// efficient concurrent readers.
pub struct BatchStore {
    epoch: OnceCell<u64>,
    last_certified_time: AtomicU64,
    db_cache: DashMap<HashValue, PersistedValue>,
    peer_quota: DashMap<PeerId, QuotaManager>,
    expirations: Mutex<TimeExpirations<HashValue>>,
    db: Arc<dyn QuorumStoreStorage>,
    memory_quota: usize,
    db_quota: usize,
    batch_quota: usize,
    validator_signer: ValidatorSigner,
    persist_subscribers: DashMap<HashValue, Vec<oneshot::Sender<PersistedValue>>>,
    expiration_buffer_usecs: u64,
}

impl BatchStore {
    pub(crate) fn new(
        epoch: u64,
        is_new_epoch: bool,
        last_certified_time: u64,
        db: Arc<dyn QuorumStoreStorage>,
        memory_quota: usize,
        db_quota: usize,
        batch_quota: usize,
        validator_signer: ValidatorSigner,
        expiration_buffer_usecs: u64,
    ) -> Self {
        let db_clone = db.clone();
        let batch_store = Self {
            epoch: OnceCell::with_value(epoch),
            last_certified_time: AtomicU64::new(last_certified_time),
            db_cache: DashMap::new(),
            peer_quota: DashMap::new(),
            expirations: Mutex::new(TimeExpirations::new()),
            db,
            memory_quota,
            db_quota,
            batch_quota,
            validator_signer,
            persist_subscribers: DashMap::new(),
            expiration_buffer_usecs,
        };

        if is_new_epoch {
            tokio::task::spawn_blocking(move || {
                Self::gc_previous_epoch_batches_from_db(db_clone, epoch);
            });
        } else {
            Self::populate_cache_and_gc_expired_batches(
                db_clone,
                epoch,
                last_certified_time,
                expiration_buffer_usecs,
                &batch_store,
            );
        }

        batch_store
    }

    fn gc_previous_epoch_batches_from_db(db: Arc<dyn QuorumStoreStorage>, current_epoch: u64) {
        let db_content = db.get_all_batches().expect("failed to read data from db");
        info!(
            epoch = current_epoch,
            "QS: Read batches from storage. Len: {}",
            db_content.len(),
        );

        let mut expired_keys = Vec::new();
        for (digest, value) in db_content {
            let epoch = value.epoch();

            trace!(
                "QS: Batchreader recovery content epoch {:?}, digest {}",
                epoch,
                digest
            );

            if epoch < current_epoch {
                expired_keys.push(digest);
            }
        }

        info!(
            "QS: Batch store bootstrap expired keys len {}",
            expired_keys.len()
        );
        db.delete_batches(expired_keys)
            .expect("Deletion of expired keys should not fail");
    }

    fn populate_cache_and_gc_expired_batches(
        db: Arc<dyn QuorumStoreStorage>,
        current_epoch: u64,
        last_certified_time: u64,
        expiration_buffer_usecs: u64,
        batch_store: &BatchStore,
    ) {
        let db_content = db.get_all_batches().expect("failed to read data from db");
        info!(
            epoch = current_epoch,
            "QS: Read batches from storage. Len: {}, Last Cerified Time: {}",
            db_content.len(),
            last_certified_time
        );

        let mut expired_keys = Vec::new();
        for (digest, value) in db_content {
            let expiration = value.expiration().saturating_sub(expiration_buffer_usecs);

            trace!(
                "QS: Batchreader recovery content exp {:?}, digest {}",
                expiration,
                digest
            );

            if last_certified_time >= expiration {
                expired_keys.push(digest);
            } else {
                batch_store
                    .insert_to_cache(&value)
                    .expect("Storage limit exceeded upon BatchReader construction");
            }
        }

        info!(
            "QS: Batch store bootstrap expired keys len {}",
            expired_keys.len()
        );
        tokio::task::spawn_blocking(move || {
            db.delete_batches(expired_keys)
                .expect("Deletion of expired keys should not fail");
        });
    }

    fn epoch(&self) -> u64 {
        *self.epoch.get().expect("Epoch should always be set")
    }

    fn free_quota(&self, value: PersistedValue) {
        let mut quota_manager = self
            .peer_quota
            .get_mut(&value.author())
            .expect("No QuotaManager for batch author");
        quota_manager.free_quota(value.num_bytes() as usize, value.payload_storage_mode());
    }

    // Inserts a PersistedValue into the in-memory db_cache. If an entry with a higher
    // value is already in the db_cache, Ok(false) is returned. If there was no entry
    // Ok(true) is returned after the successful insertion. Finally, the method returns
    // an error if storage quota is exceeded (if in-memory quota is exceeded,
    // only the metadata is stored in the db-cache).
    // Note: holds db_cache entry lock (due to DashMap), while accessing peer_quota
    // DashMap. Hence, peer_quota reference should never be held while accessing the
    // db_cache to avoid the deadlock (if needed, order is db_cache, then peer_quota).
    pub(crate) fn insert_to_cache(&self, value: &PersistedValue) -> anyhow::Result<bool> {
        let digest = *value.digest();
        let author = value.author();
        let expiration_time = value.expiration();

        {
            // Acquire dashmap internal lock on the entry corresponding to the digest.
            let cache_entry = self.db_cache.entry(digest);

            if let Occupied(entry) = &cache_entry {
                match entry.get().expiration().cmp(&expiration_time) {
                    std::cmp::Ordering::Equal => return Ok(false),
                    std::cmp::Ordering::Greater => {
                        debug!(
                            "QS: already have the digest with higher expiration {}",
                            digest
                        );
                        return Ok(false);
                    },
                    std::cmp::Ordering::Less => {},
                }
            };
            let value_to_be_stored = if self
                .peer_quota
                .entry(author)
                .or_insert(QuotaManager::new(
                    self.db_quota,
                    self.memory_quota,
                    self.batch_quota,
                ))
                .update_quota(value.num_bytes() as usize)?
                == StorageMode::PersistedOnly
            {
                PersistedValue::new(value.batch_info().clone(), None)
            } else {
                value.clone()
            };

            match cache_entry {
                Occupied(entry) => {
                    let (k, prev_value) = entry.replace_entry(value_to_be_stored);
                    debug_assert!(k == digest);
                    self.free_quota(prev_value);
                },
                Vacant(slot) => {
                    slot.insert(value_to_be_stored);
                },
            }
        }

        // Add expiration for the inserted entry, no need to be atomic w. insertion.
        #[allow(clippy::unwrap_used)]
        {
            self.expirations.lock().add_item(digest, expiration_time);
        }
        Ok(true)
    }

    pub(crate) fn save(&self, value: &PersistedValue) -> anyhow::Result<bool> {
        let last_certified_time = self.last_certified_time();
        if value.expiration() > last_certified_time {
            fail_point!("quorum_store::save", |_| {
                // Skip caching and storing value to the db
                Ok(false)
            });
            counters::GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_TIME_WHEN_SAVE.observe(
                Duration::from_micros(value.expiration() - last_certified_time).as_secs_f64(),
            );

            return self.insert_to_cache(value);
        }
        counters::NUM_BATCH_EXPIRED_WHEN_SAVE.inc();
        bail!(
            "Incorrect expiration {} in epoch {}, last committed timestamp {}",
            value.expiration(),
            self.epoch(),
            last_certified_time,
        );
    }

    // pub(crate) for testing
    #[allow(clippy::unwrap_used)]
    pub(crate) fn clear_expired_payload(&self, certified_time: u64) -> Vec<HashValue> {
        // To help slow nodes catch up via execution without going to state sync we keep the blocks for 60 extra seconds
        // after the expiration time. This will help remote peers fetch batches that just expired but are within their
        // execution window.
        let expiration_time = certified_time.saturating_sub(self.expiration_buffer_usecs);
        let expired_digests = self.expirations.lock().expire(expiration_time);
        let mut ret = Vec::new();
        for h in expired_digests {
            let removed_value = match self.db_cache.entry(h) {
                Occupied(entry) => {
                    // We need to check up-to-date expiration again because receiving the same
                    // digest with a higher expiration would update the persisted value and
                    // effectively extend the expiration.
                    if entry.get().expiration() <= expiration_time {
                        self.persist_subscribers.remove(entry.get().digest());
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

    fn generate_signed_batch_info(
        &self,
        batch_info: BatchInfo,
    ) -> Result<SignedBatchInfo, CryptoMaterialError> {
        fail_point!("quorum_store::create_invalid_signed_batch_info", |_| {
            Ok(SignedBatchInfo::new_with_signature(
                batch_info.clone(),
                self.validator_signer.author(),
                aptos_crypto::bls12381::Signature::dummy_signature(),
            ))
        });
        SignedBatchInfo::new(batch_info, &self.validator_signer)
    }

    fn persist_inner(&self, persist_request: PersistedValue) -> Option<SignedBatchInfo> {
        match self.save(&persist_request) {
            Ok(needs_db) => {
                let batch_info = persist_request.batch_info().clone();
                trace!("QS: sign digest {}", persist_request.digest());
                if needs_db {
                    #[allow(clippy::unwrap_in_result)]
                    self.db
                        .save_batch(persist_request)
                        .expect("Could not write to DB");
                }
                self.generate_signed_batch_info(batch_info).ok()
            },

            Err(e) => {
                debug!("QS: failed to store to cache {:?}", e);
                None
            },
        }
    }

    pub fn update_certified_timestamp(&self, certified_time: u64) {
        trace!("QS: batch reader updating time {:?}", certified_time);
        self.last_certified_time
            .fetch_max(certified_time, Ordering::SeqCst);

        let expired_keys = self.clear_expired_payload(certified_time);
        if let Err(e) = self.db.delete_batches(expired_keys) {
            debug!("Error deleting batches: {:?}", e)
        }
    }

    fn last_certified_time(&self) -> u64 {
        self.last_certified_time.load(Ordering::Relaxed)
    }

    fn get_batch_from_db(&self, digest: &HashValue) -> ExecutorResult<PersistedValue> {
        counters::GET_BATCH_FROM_DB_COUNT.inc();

        match self.db.get_batch(digest) {
            Ok(Some(value)) => Ok(value),
            Ok(None) | Err(_) => {
                warn!("Could not get batch from db");
                Err(ExecutorError::CouldNotGetData)
            },
        }
    }

    pub(crate) fn get_batch_from_local(
        &self,
        digest: &HashValue,
    ) -> ExecutorResult<PersistedValue> {
        if let Some(value) = self.db_cache.get(digest) {
            if value.payload_storage_mode() == StorageMode::PersistedOnly {
                self.get_batch_from_db(digest)
            } else {
                // Available in memory.
                Ok(value.clone())
            }
        } else {
            Err(ExecutorError::CouldNotGetData)
        }
    }

    /// This calls lets the caller subscribe to a batch being added to the batch store.
    /// This can be useful in cases where there are multiple flows to add a batch (like
    /// direct from author batch / batch requester fetch) to the batch store and either
    /// flow needs to subscribe to the other.
    fn subscribe(&self, digest: HashValue) -> oneshot::Receiver<PersistedValue> {
        let (tx, rx) = oneshot::channel();
        self.persist_subscribers.entry(digest).or_default().push(tx);

        // This is to account for the race where this subscribe call happens after the
        // persist call.
        if let Ok(value) = self.get_batch_from_local(&digest) {
            self.notify_subscribers(value)
        }

        rx
    }

    fn notify_subscribers(&self, value: PersistedValue) {
        if let Some((_, subscribers)) = self.persist_subscribers.remove(value.digest()) {
            for subscriber in subscribers {
                subscriber.send(value.clone()).ok();
            }
        }
    }
}

impl BatchWriter for BatchStore {
    fn persist(&self, persist_requests: Vec<PersistedValue>) -> Vec<SignedBatchInfo> {
        let mut signed_infos = vec![];
        for persist_request in persist_requests.into_iter() {
            if let Some(signed_info) = self.persist_inner(persist_request.clone()) {
                self.notify_subscribers(persist_request);
                signed_infos.push(signed_info);
            }
        }
        signed_infos
    }
}

pub trait BatchReader: Send + Sync {
    /// Check if the batch corresponding to the digest exists, return the batch author if true
    fn exists(&self, digest: &HashValue) -> Option<PeerId>;

    fn get_batch(
        &self,
        batch_info: BatchInfo,
        signers: Vec<PeerId>,
    ) -> Shared<Pin<Box<dyn Future<Output = ExecutorResult<Vec<SignedTransaction>>> + Send>>>;

    fn update_certified_timestamp(&self, certified_time: u64);
}

struct BatchFetchUnit {
    responders: Arc<Mutex<BTreeSet<PeerId>>>,
    fut: Shared<Pin<Box<dyn Future<Output = ExecutorResult<Vec<SignedTransaction>>> + Send>>>,
}

pub struct BatchReaderImpl<T> {
    batch_store: Arc<BatchStore>,
    batch_requester: Arc<BatchRequester<T>>,
    inflight_fetch_requests: Arc<Mutex<HashMap<HashValue, BatchFetchUnit>>>,
}

impl<T: QuorumStoreSender + Clone + Send + Sync + 'static> BatchReaderImpl<T> {
    pub(crate) fn new(batch_store: Arc<BatchStore>, batch_requester: BatchRequester<T>) -> Self {
        Self {
            batch_store,
            batch_requester: Arc::new(batch_requester),
            inflight_fetch_requests: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_or_fetch_batch(
        &self,
        batch_info: BatchInfo,
        responders: Vec<PeerId>,
    ) -> Shared<Pin<Box<dyn Future<Output = ExecutorResult<Vec<SignedTransaction>>> + Send>>> {
        let mut responders = responders.into_iter().collect();

        self.inflight_fetch_requests
            .lock()
            .entry(*batch_info.digest())
            .and_modify(|fetch_unit| {
                fetch_unit.responders.lock().append(&mut responders);
            })
            .or_insert_with(|| {
                let responders = Arc::new(Mutex::new(responders));
                let responders_clone = responders.clone();

                let inflight_requests_clone = self.inflight_fetch_requests.clone();
                let batch_store = self.batch_store.clone();
                let requester = self.batch_requester.clone();

                let fut = async move {
                    let batch_digest = *batch_info.digest();
                    defer!({
                        inflight_requests_clone.lock().remove(&batch_digest);
                    });
                    if let Ok(mut value) = batch_store.get_batch_from_local(&batch_digest) {
                        Ok(value.take_payload().expect("Must have payload"))
                    } else {
                        // Quorum store metrics
                        counters::MISSED_BATCHES_COUNT.inc();
                        let subscriber_rx = batch_store.subscribe(*batch_info.digest());
                        let payload = requester
                            .request_batch(
                                batch_digest,
                                batch_info.expiration(),
                                responders,
                                subscriber_rx,
                            )
                            .await?;
                        batch_store
                            .persist(vec![PersistedValue::new(batch_info, Some(payload.clone()))]);
                        Ok(payload)
                    }
                }
                .boxed()
                .shared();

                tokio::spawn(fut.clone());

                BatchFetchUnit {
                    responders: responders_clone,
                    fut,
                }
            })
            .fut
            .clone()
    }
}

impl<T: QuorumStoreSender + Clone + Send + Sync + 'static> BatchReader for BatchReaderImpl<T> {
    fn exists(&self, digest: &HashValue) -> Option<PeerId> {
        self.batch_store
            .get_batch_from_local(digest)
            .map(|v| v.author())
            .ok()
    }

    fn get_batch(
        &self,
        batch_info: BatchInfo,
        responders: Vec<PeerId>,
    ) -> Shared<Pin<Box<dyn Future<Output = ExecutorResult<Vec<SignedTransaction>>> + Send>>> {
        self.get_or_fetch_batch(batch_info, responders)
    }

    fn update_certified_timestamp(&self, certified_time: u64) {
        self.batch_store.update_certified_timestamp(certified_time);
    }
}

pub trait BatchWriter: Send + Sync {
    fn persist(&self, persist_requests: Vec<PersistedValue>) -> Vec<SignedBatchInfo>;
}
