// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network_interface::ConsensusMsg;
use crate::quorum_store::types::{Payload, Batch, PersistedValue, ProofOfStore};
use crate::{
    network::NetworkSender,
    quorum_store::{
        batch_requester::BatchRequester,
        batch_store::{BatchStoreCommand, LogicalTime},
    },
};
use aptos_crypto::HashValue;
use aptos_types::{validator_signer::ValidatorSigner, PeerId};
use consensus_types::common::{Round};
use dashmap::DashMap;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver as SyncReceiver, RecvTimeoutError, SyncSender},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::{mpsc::Sender, oneshot};


// Make configuration parameter (passed to QuorumStore).
/// Maximum number of rounds in the future from local prospective to hold batches for.
const MAX_BATCH_EXPIRY_ROUND_GAP: Round = 20;

#[allow(dead_code)]
pub(crate) enum BatchReaderCommand {
    GetBatchForPeer(HashValue, PeerId),
    GetBatchForSelf(ProofOfStore, oneshot::Sender<Payload>),
    BatchResponse(HashValue, Payload),
}

pub(crate) enum StoreType {
    MemoryCache,
    PersistOnly,
    LimitedExceed,
}

pub(crate) struct QuotaManger {
    memory_balance: usize,
    db_balance: usize,
    db_quota: usize,
    memory_quota: usize,
}

impl QuotaManger {
    fn new(max_db_balance: usize, max_memory_balance: usize) -> Self {
        assert!(max_db_balance >= max_memory_balance);
        Self {
            memory_balance: 0,
            db_balance: 0,
            db_quota: max_db_balance,
            memory_quota: max_memory_balance,
        }
    }

    pub(crate) fn store_type(&mut self, num_of_bytes: usize) -> StoreType {
        if self.memory_balance + num_of_bytes <= self.memory_quota {
            self.memory_balance = self.memory_balance + num_of_bytes;
            self.db_balance = self.db_balance + num_of_bytes;
            StoreType::MemoryCache
        } else if self.db_balance + num_of_bytes <= self.db_quota {
            self.db_balance = self.db_balance + num_of_bytes;
            StoreType::PersistOnly
        } else {
            StoreType::LimitedExceed
        }
    }

    pub(crate) fn free_quota(&mut self, num_of_bytes: usize, store_type: StoreType) {
        match store_type {
            StoreType::MemoryCache => {
                self.memory_balance = self.memory_balance - num_of_bytes;
            }
            StoreType::PersistOnly => {
                self.db_balance = self.db_balance - num_of_bytes;
            }
            _ => {
                unreachable!();
            }
        }
    }
}

/// Provides in memory representation of stored batches (strong cache), allowing
/// efficient concurrent readers.
pub struct BatchReader {
    epoch: OnceCell<u64>,
    my_peer_id: PeerId,
    last_committed_round: AtomicU64,
    db_cache: DashMap<HashValue, PersistedValue>,
    peer_quota: DashMap<PeerId, QuotaManger>,
    expirations: Mutex<BinaryHeap<(Reverse<Round>, HashValue)>>,
    batch_store_tx: Sender<BatchStoreCommand>,
    self_tx: SyncSender<BatchReaderCommand>,
    max_execution_round_lag: Round,
    memory_quota: usize,
    db_quota: usize,
}

impl BatchReader {
    pub(crate) fn new(
        epoch: u64,
        last_committed_round: Round,
        db_content: HashMap<HashValue, PersistedValue>,
        my_peer_id: PeerId,
        batch_store_tx: Sender<BatchStoreCommand>,
        self_tx: SyncSender<BatchReaderCommand>,
        max_execution_round_lag: Round,
        memory_quota: usize,
        db_quota: usize,
    ) -> (Self, Vec<HashValue>) {
        let db_cache = DashMap::new();
        let peer_quota = DashMap::new();
        let expirations = Mutex::new(BinaryHeap::new());

        let self_ob = Self {
            epoch: OnceCell::with_value(epoch),
            my_peer_id,
            last_committed_round: AtomicU64::new(last_committed_round),
            db_cache,
            peer_quota,
            expirations,
            batch_store_tx,
            self_tx,
            max_execution_round_lag,
            memory_quota,
            db_quota,
        };

        let mut expired_keys = Vec::new();
        for (digest, value) in db_content {
            let expiration = value.expiration;
            assert!(epoch >= expiration.epoch());
            if epoch > expiration.epoch()
                || last_committed_round - MAX_BATCH_EXPIRY_ROUND_GAP > expiration.round()
            {
                expired_keys.push(digest);
            } else {
                let ret = self_ob.update_cache(digest, value);
                assert!(ret);
            }
        }

        (self_ob, expired_keys)
    }

    fn epoch(&self) -> u64 {
        *self.epoch.get().unwrap()
    }

    // returns true if value needs to be persisted
    fn update_cache(&self, digest: HashValue, mut value: PersistedValue) -> bool {
        let author = value.author;
        let mut entry = self
            .peer_quota
            .entry(author)
            .or_insert(QuotaManger::new(self.db_quota, self.memory_quota));

        match entry.store_type(value.num_bytes) {
            StoreType::MemoryCache => {}
            StoreType::PersistOnly => {
                value.remove_payload();
            }
            StoreType::LimitedExceed => {
                return false;
            }
        }

        self.expirations
            .lock()
            .unwrap()
            .push((Reverse(value.expiration.round()), digest));
        self.db_cache.insert(digest, value);
        true
    }

    pub(crate) fn save(&self, digest: HashValue, value: PersistedValue) -> bool {
        if value.expiration.epoch() == self.epoch()
            && value.expiration.round() > self.last_committed_round()
            && value.expiration.round() <= self.last_committed_round() + MAX_BATCH_EXPIRY_ROUND_GAP
        {
            if let Some(prev_value) = self.db_cache.get(&digest) {
                if prev_value.expiration.round() < value.expiration.round() {
                    self.remove_value(&digest);
                } else {
                    return false;
                }
            }
            self.update_cache(digest, value)
        } else {
            false
        }
    }

    fn clear_expired_payload(&self, certified_time: LogicalTime) -> Vec<HashValue> {
        assert_eq!(
            certified_time.epoch(),
            self.epoch(),
            "Execution epoch inconsistent with BatchReader"
        );

        let expired_round = if certified_time.round() >= self.max_execution_round_lag {
            certified_time.round() - self.max_execution_round_lag
        } else {
            0
        };

        let mut ret = Vec::new();
        let mut expirations = self.expirations.lock().unwrap();
        loop {
            if let Some((Reverse(r), _)) = expirations.peek() {
                if *r <= expired_round {
                    let (_, h) = expirations.pop().unwrap();
                    self.remove_value(&h);
                    ret.push(h);
                }
            } else {
                break;
            }
        }
        ret
    }

    fn remove_value(&self, digest: &HashValue) {
        let (_, persisted_value) = self.db_cache.remove(digest).unwrap();
        let mut quota_manger = self.peer_quota.get_mut(&persisted_value.author).unwrap();
        if persisted_value.maybe_payload.is_some() {
            quota_manger.free_quota(persisted_value.num_bytes, StoreType::MemoryCache);
        } else {
            quota_manger.free_quota(persisted_value.num_bytes, StoreType::PersistOnly);
        }
    }

    // TODO: maybe check the epoch to stop communicating on epoch change.
    // TODO: make sure state-sync also sends the message.
    // TODO: make sure message is sent execution re-starts (will also clean)
    #[allow(dead_code)]
    pub async fn update_certified_round(&self, certified_time: LogicalTime) {
        let prev_round = self
            .last_committed_round
            .fetch_max(certified_time.round(), Ordering::SeqCst);
        assert!(
            prev_round < certified_time.round(),
            "Non-increasing executed rounds reported to BatchStore"
        );
        let expired_keys = self.clear_expired_payload(certified_time);
        self.batch_store_tx
            .send(BatchStoreCommand::Clean(expired_keys))
            .await
            .expect("Failed to send to BatchStore");
    }

    fn last_committed_round(&self) -> Round {
        self.last_committed_round.load(Ordering::Relaxed)
    }

    // TODO: maybe check the epoch to stop communicating on epoch change.
    #[allow(dead_code)]
    pub async fn get_batch(&self, proof: ProofOfStore, ret_tx: oneshot::Sender<Payload>) {
        if let Some(value) = self.db_cache.get(&proof.digest()) {
            if value.maybe_payload.is_some() {
                ret_tx
                    .send(value.maybe_payload.clone().unwrap())
                    .expect("Receiver of requested batch is not available");
            } else {
                self.batch_store_tx
                    .send(BatchStoreCommand::BatchRequest(
                        proof.digest().clone(),
                        self.my_peer_id,
                        Some(ret_tx),
                    ))
                    .await
                    .expect("Failed to send to BatchStore");
            }
        } else {
            self.self_tx
                .send(BatchReaderCommand::GetBatchForSelf(proof, ret_tx))
                .expect("Batch Reader Receiver is not available");
        }
    }

    pub(crate) async fn start(
        &self,
        batch_reader_rx: SyncReceiver<BatchReaderCommand>,
        network_sender: NetworkSender,
        signer: Arc<ValidatorSigner>,
        request_num_peers: usize,
        request_timeout_ms: usize,
    ) {
        let mut batch_requester = BatchRequester::new(
            self.epoch(),
            self.my_peer_id,
            request_num_peers,
            request_timeout_ms,
            network_sender.clone(),
        );

        loop {
            // SyncReceiver holds a lock, so receive a message first to create short-lived borrow.
            let cmd = match batch_reader_rx
                .recv_timeout(Duration::from_millis((request_timeout_ms / 10) as u64))  // TODO: think about the right smaller timeout
            {
                Ok(cmd) => Some(cmd),
                Err(err) => match err {
                    RecvTimeoutError::Timeout => None,
                    _ => break,
                },
            };
            if let Some(cmd) = cmd {
                match cmd {
                    BatchReaderCommand::GetBatchForPeer(digest, peer_id) => {
                        //TODO: check if needs to read from db - probably storage will send directly?
                        if let Some(value) = self.db_cache.get(&digest) {
                            if value.maybe_payload.is_some() {
                                let batch = Batch::new(
                                    self.epoch(),
                                    self.my_peer_id,
                                    digest,
                                    Some(value.maybe_payload.as_ref().unwrap().clone()),
                                    signer.clone(),
                                );
                                let msg = ConsensusMsg::BatchMsg(Box::new(batch));
                                network_sender.send(msg, vec![peer_id]).await;
                            } else {
                                self.batch_store_tx
                                    .send(BatchStoreCommand::BatchRequest(digest, peer_id, None))
                                    .await
                                    .expect("Failed to send to BatchStore");
                            }
                        } //TODO: consider returning Nack
                    }
                    BatchReaderCommand::GetBatchForSelf(proof, ret_tx) => {
                        batch_requester
                            .add_request(
                                proof.digest().clone(),
                                proof.shuffled_signers(),
                                ret_tx,
                                signer.clone(),
                            )
                            .await;
                        // TODO: actual send over network and local match one-shot / time-out logic.
                    }
                    BatchReaderCommand::BatchResponse(digest, payload) => {
                        batch_requester.serve_request(digest, payload);
                    }
                }
            }
            batch_requester.handle_timeouts(signer.clone()).await;
        }
    }
}
