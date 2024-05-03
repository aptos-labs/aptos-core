// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    quorum_store::{batch_store::BatchReader, quorum_store_coordinator::CoordinatorCommand},
};
use aptos_consensus_types::{
    block::Block,
    common::{DataStatus, Payload, ProofWithData},
    proof_of_store::ProofOfStore,
};
use aptos_crypto::HashValue;
use aptos_executor_types::{ExecutorError::DataNotFound, *};
use aptos_logger::prelude::*;
use aptos_types::transaction::{SignedTransaction, Transaction};
use futures::channel::mpsc::Sender;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::sync::Arc;
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
        block_timestamp: u64,
        batch_reader: Arc<dyn BatchReader>,
    ) -> Vec<(
        HashValue,
        oneshot::Receiver<ExecutorResult<Vec<SignedTransaction>>>,
    )> {
        let mut receivers = Vec::new();
        for pos in proofs {
            trace!(
                "QSE: requesting pos {:?}, digest {}, time = {}",
                pos,
                pos.digest(),
                block_timestamp
            );
            if block_timestamp <= pos.expiration() {
                receivers.push((*pos.digest(), batch_reader.get_batch(pos)));
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
                            .map(|proof| proof.info().clone())
                            .collect::<Vec<_>>(),
                        Payload::InQuorumStoreWithLimit(proof_with_status) => proof_with_status
                            .proof_with_data
                            .proofs
                            .iter()
                            .map(|proof| proof.info().clone())
                            .collect::<Vec<_>>(),
                        Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                            inline_batches
                                .iter()
                                .map(|(batch_info, _)| batch_info.clone())
                                .chain(
                                    proof_with_data
                                        .proofs
                                        .iter()
                                        .map(|proof| proof.info().clone()),
                                )
                                .collect::<Vec<_>>()
                        },
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
        let request_txns_and_update_status =
            move |proof_with_status: &ProofWithData, batch_reader: Arc<dyn BatchReader>| {
                let receivers = PayloadManager::request_transactions(
                    proof_with_status.proofs.clone(),
                    timestamp,
                    batch_reader.clone(),
                );
                proof_with_status
                    .status
                    .lock()
                    .replace(DataStatus::Requested(receivers));
            };

        match self {
            PayloadManager::DirectMempool => {},
            PayloadManager::InQuorumStore(batch_reader, _) => match payload {
                Payload::InQuorumStore(proof_with_status) => {
                    request_txns_and_update_status(proof_with_status, batch_reader.clone());
                },
                Payload::InQuorumStoreWithLimit(proof_with_data) => {
                    request_txns_and_update_status(
                        &proof_with_data.proof_with_data,
                        batch_reader.clone(),
                    );
                },
                Payload::QuorumStoreInlineHybrid(_, proof_with_data, _) => {
                    request_txns_and_update_status(proof_with_data, batch_reader.clone());
                },
                Payload::DirectMempool(_) => {
                    unreachable!()
                },
            },
        }
    }

    /// Extract transaction from a given block
    /// Assumes it is never called for the same block concurrently. Otherwise status can be None.
    pub async fn get_transactions(
        &self,
        block: &Block,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<usize>)> {
        let payload = match block.payload() {
            Some(p) => p,
            None => return Ok((Vec::new(), None)),
        };

        async fn process_payload(
            proof_with_data: &ProofWithData,
            batch_reader: Arc<dyn BatchReader>,
            block: &Block,
        ) -> ExecutorResult<Vec<SignedTransaction>> {
            let status = proof_with_data.status.lock().take();
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
                    for (digest, rx) in receivers {
                        match rx.await {
                            Err(e) => {
                                // We probably advanced epoch already.
                                warn!(
                                    "Oneshot channel to get a batch was dropped with error {:?}",
                                    e
                                );
                                let new_receivers = PayloadManager::request_transactions(
                                    proof_with_data.proofs.clone(),
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
        }

        match (self, payload) {
            (PayloadManager::DirectMempool, Payload::DirectMempool(txns)) => Ok((
                txns.par_iter()
                    .with_min_len(500)
                    .map(|t| t.clone())
                    .collect::<Vec<_>>(),
                None,
            )),
            (
                PayloadManager::InQuorumStore(batch_reader, _),
                Payload::InQuorumStore(proof_with_data),
            ) => Ok((
                process_payload(proof_with_data, batch_reader.clone(), block).await?,
                None,
            )),
            (
                PayloadManager::InQuorumStore(batch_reader, _),
                Payload::InQuorumStoreWithLimit(proof_with_data),
            ) => Ok((
                process_payload(
                    &proof_with_data.proof_with_data,
                    batch_reader.clone(),
                    block,
                )
                .await?,
                proof_with_data.max_txns_to_execute,
            )),
            (
                PayloadManager::InQuorumStore(batch_reader, _),
                Payload::QuorumStoreInlineHybrid(
                    inline_batches,
                    proof_with_data,
                    max_txns_to_execute,
                ),
            ) => Ok((
                {
                    let mut all_txns =
                        process_payload(proof_with_data, batch_reader.clone(), block).await?;
                    all_txns.append(
                        &mut inline_batches
                            .iter()
                            // TODO: Can clone be avoided here?
                            .flat_map(|(_batch_info, txns)| txns.clone())
                            .collect(),
                    );
                    all_txns
                },
                *max_txns_to_execute,
            )),
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
