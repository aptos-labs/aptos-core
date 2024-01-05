// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    quorum_store::{batch_store::BatchReader, quorum_store_coordinator::CoordinatorCommand},
};
use aptos_consensus_types::{
    block::Block,
    common::{DataStatus, Payload},
    proof_of_store::{ProofOfStore, ProposedBatch},
};
use aptos_crypto::HashValue;
use aptos_executor_types::{ExecutorError::DataNotFound, *};
use aptos_logger::prelude::*;
use aptos_types::transaction::SignedTransaction;
use futures::channel::mpsc::Sender;
use std::{ops::Range, sync::Arc};
use tokio::sync::oneshot;

pub trait TPayloadManager: Send + Sync {
    fn prefetch_payload_data(&self, payload: &Payload, timestamp: u64);
}

/// Responsible to extract the transactions out of the payload and notify QuorumStore about commits.
/// If QuorumStore is enabled, has to ask BatchReader for the transaction behind the proofs of availability in the payload.
pub enum PayloadManager {
    DirectMempool,
    InQuorumStore(Arc<dyn BatchReader>, Sender<CoordinatorCommand>),
}

impl TPayloadManager for PayloadManager {
    fn prefetch_payload_data(&self, payload: &Payload, timestamp: u64) {
        self.prefetch_payload_data(payload, timestamp);
    }
}

impl PayloadManager {
    fn request_transactions(
        proofs: Vec<ProofOfStore>,
        ranges: Vec<Range<u64>>,
        block_timestamp: u64,
        batch_reader: Arc<dyn BatchReader>,
    ) -> Vec<(
        HashValue,
        oneshot::Receiver<ExecutorResult<Vec<SignedTransaction>>>,
        Range<u64>,
    )> {
        let mut receivers = Vec::new();
        for (pos, range) in proofs.into_iter().zip(ranges.into_iter()) {
            trace!(
                "QSE: requesting pos {:?}, digest {}, time = {}",
                pos,
                pos.digest(),
                block_timestamp
            );
            if block_timestamp <= pos.expiration() {
                receivers.push((*pos.digest(), batch_reader.get_batch(pos), range));
            } else {
                debug!("QSE: skipped expired pos {}", pos.digest());
            }
        }
        receivers
    }

    ///Pass commit information to BatchReader and QuorumStore wrapper for their internal cleanups.
    pub fn notify_commit(&self, block_timestamp: u64, payloads: Vec<Payload>) {
        match self {
            PayloadManager::DirectMempool => {},
            PayloadManager::InQuorumStore(batch_reader, coordinator_tx) => {
                batch_reader.update_certified_timestamp(block_timestamp);

                let batches: Vec<_> = payloads
                    .into_iter()
                    .flat_map(|payload| match payload {
                        Payload::DirectMempool(_) => {
                            unreachable!("InQuorumStore should be used");
                        },
                        Payload::InQuorumStore(proof_with_status) => proof_with_status
                            .proofs
                            .iter()
                            .map(|proof| ProposedBatch::new(proof.info().clone()))
                            .collect::<Vec<_>>(),
                        Payload::InQuorumStoreV2(proofs) => proofs
                            .proof_with_data
                            .proofs
                            .iter()
                            .zip(proofs.ranges.iter())
                            .map(|(proof, range)| {
                                ProposedBatch::new_with_range(proof.info().clone(), range.clone())
                            })
                            .collect::<Vec<_>>(),
                    })
                    .collect();

                let mut tx = coordinator_tx.clone();

                if let Err(e) = tx.try_send(CoordinatorCommand::CommitNotification(
                    block_timestamp,
                    batches,
                )) {
                    warn!(
                        "CommitNotification failed. Is the epoch shutting down? error: {}",
                        e
                    );
                }
            },
        }
    }

    /// Called from consensus to pre-fetch the transaction behind the batches in the block.
    pub fn prefetch_payload_data(&self, payload: &Payload, timestamp: u64) {
        match self {
            PayloadManager::DirectMempool => {},
            PayloadManager::InQuorumStore(batch_reader, _) => match payload {
                Payload::InQuorumStore(proof_with_status) => {
                    let ranges: Vec<_> = proof_with_status
                        .proofs
                        .iter()
                        .map(|p| 0..p.num_txns())
                        .collect();
                    if proof_with_status.status.lock().is_none() {
                        let receivers = PayloadManager::request_transactions(
                            proof_with_status.proofs.clone(),
                            ranges,
                            timestamp,
                            batch_reader.clone(),
                        );
                        proof_with_status
                            .status
                            .lock()
                            .replace(DataStatus::Requested(receivers));
                    }
                },
                Payload::InQuorumStoreV2(proofs) => {
                    if proofs.proof_with_data.status.lock().is_none() {
                        let receivers = PayloadManager::request_transactions(
                            proofs.proof_with_data.proofs.clone(),
                            proofs.ranges.clone(),
                            timestamp,
                            batch_reader.clone(),
                        );
                        proofs
                            .proof_with_data
                            .status
                            .lock()
                            .replace(DataStatus::Requested(receivers));
                    }
                },
                Payload::DirectMempool(_) => {
                    unreachable!()
                },
            },
        }
    }

    /// Extract transaction from a given block
    /// Assumes it is never called for the same block concurrently. Otherwise status can be None.
    pub async fn get_transactions(&self, block: &Block) -> ExecutorResult<Vec<SignedTransaction>> {
        let payload = match block.payload() {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };

        match (self, payload) {
            (PayloadManager::DirectMempool, Payload::DirectMempool(txns)) => Ok(txns.clone()),
            (
                PayloadManager::InQuorumStore(batch_reader, _),
                Payload::InQuorumStore(proof_with_data),
            ) => {
                let status = proof_with_data.status.lock().take();
                let ranges: Vec<_> = proof_with_data
                    .proofs
                    .iter()
                    .map(|p| 0..p.num_txns())
                    .collect();
                match status.expect("Should have been updated before.") {
                    DataStatus::Cached(data) => {
                        counters::QUORUM_BATCH_READY_COUNT.inc();
                        proof_with_data
                            .status
                            .lock()
                            .replace(DataStatus::Cached(data.clone()));
                        Ok(data)
                    },
                    DataStatus::Requested(receivers) => {
                        let _timer = counters::BATCH_WAIT_DURATION.start_timer();
                        let mut vec_ret = Vec::new();
                        if !receivers.is_empty() {
                            debug!(
                                "QSE: waiting for data on {} receivers, block_round {}",
                                receivers.len(),
                                block.round()
                            );
                        }
                        for (digest, rx, _) in receivers {
                            match rx.await {
                                Err(e) => {
                                    // We probably advanced epoch already.
                                    warn!("Oneshot channel to get a batch was dropped with error {:?}", e);
                                    let new_receivers = PayloadManager::request_transactions(
                                        proof_with_data.proofs.clone(),
                                        ranges,
                                        block.timestamp_usecs(),
                                        batch_reader.clone(),
                                    );
                                    // Could not get all data so requested again
                                    proof_with_data
                                        .status
                                        .lock()
                                        .replace(DataStatus::Requested(new_receivers));
                                    return Err(DataNotFound(digest));
                                },
                                Ok(Ok(data)) => {
                                    vec_ret.push(data);
                                },
                                Ok(Err(e)) => {
                                    let new_receivers = PayloadManager::request_transactions(
                                        proof_with_data.proofs.clone(),
                                        ranges,
                                        block.timestamp_usecs(),
                                        batch_reader.clone(),
                                    );
                                    // Could not get all data so requested again
                                    proof_with_data
                                        .status
                                        .lock()
                                        .replace(DataStatus::Requested(new_receivers));
                                    return Err(e);
                                },
                            }
                        }
                        let ret: Vec<SignedTransaction> = vec_ret.into_iter().flatten().collect();
                        // execution asks for the data twice, so data is cached here for the second time.
                        proof_with_data
                            .status
                            .lock()
                            .replace(DataStatus::Cached(ret.clone()));
                        Ok(ret)
                    },
                }
            },
            (PayloadManager::InQuorumStore(batch_reader, _), Payload::InQuorumStoreV2(proof)) => {
                let status = proof.proof_with_data.status.lock().take();
                match status.expect("Should have been updated before.") {
                    DataStatus::Cached(data) => {
                        counters::QUORUM_BATCH_READY_COUNT.inc();
                        proof
                            .proof_with_data
                            .status
                            .lock()
                            .replace(DataStatus::Cached(data.clone()));
                        Ok(data)
                    },
                    DataStatus::Requested(receivers) => {
                        let _timer = counters::BATCH_WAIT_DURATION.start_timer();
                        let mut vec_all = Vec::new();
                        let mut ranges = Vec::new();
                        if !receivers.is_empty() {
                            debug!(
                                "QSE: waiting for data on {} receivers, block_round {}",
                                receivers.len(),
                                block.round()
                            );
                        }
                        for (digest, rx, range) in receivers {
                            match rx.await {
                                Err(e) => {
                                    // We probably advanced epoch already.
                                    warn!("Oneshot channel to get a batch was dropped with error {:?}", e);
                                    let new_receivers = PayloadManager::request_transactions(
                                        proof.proof_with_data.proofs.clone(),
                                        proof.ranges.clone(),
                                        block.timestamp_usecs(),
                                        batch_reader.clone(),
                                    );
                                    // Could not get all data so requested again
                                    proof
                                        .proof_with_data
                                        .status
                                        .lock()
                                        .replace(DataStatus::Requested(new_receivers));
                                    return Err(DataNotFound(digest));
                                },
                                Ok(Ok(data)) => {
                                    vec_all.push(data);
                                    ranges.push(range);
                                },
                                Ok(Err(e)) => {
                                    let new_receivers = PayloadManager::request_transactions(
                                        proof.proof_with_data.proofs.clone(),
                                        proof.ranges.clone(),
                                        block.timestamp_usecs(),
                                        batch_reader.clone(),
                                    );
                                    // Could not get all data so requested again
                                    proof
                                        .proof_with_data
                                        .status
                                        .lock()
                                        .replace(DataStatus::Requested(new_receivers));
                                    return Err(e);
                                },
                            }
                        }
                        let ret: Vec<SignedTransaction> = vec_all
                            .clone()
                            .into_iter()
                            .zip(ranges.into_iter())
                            .flat_map(|(data, range)| {
                                data.into_iter()
                                    .skip(range.start as usize)
                                    .take(range.end as usize - range.start as usize)
                                    .collect::<Vec<_>>()
                            })
                            .collect();

                        let all: Vec<SignedTransaction> = vec_all.into_iter().flatten().collect();
                        // execution asks for the data twice, so data is cached here for the second time.
                        proof
                            .proof_with_data
                            .status
                            .lock()
                            .replace(DataStatus::Cached(all.clone()));
                        Ok(ret)
                    },
                }
            },
            (_, _) => unreachable!(
                "Wrong payload {} epoch {}, round {}, id {}",
                payload,
                block.block_data().epoch(),
                block.block_data().round(),
                block.id()
            ),
        }
    }
}
