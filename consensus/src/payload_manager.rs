// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        network_message::{BlockTransactionPayload, ConsensusObserverMessage},
        payload_store::BlockPayloadStatus,
        publisher::ConsensusPublisher,
    },
    counters,
    quorum_store::{batch_store::BatchReader, quorum_store_coordinator::CoordinatorCommand},
};
use aptos_consensus_types::{
    block::Block,
    common::{DataStatus, Payload, ProofWithData, Round},
    proof_of_store::{BatchInfo, ProofOfStore},
};
use aptos_crypto::HashValue;
use aptos_executor_types::{ExecutorError::DataNotFound, *};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::transaction::SignedTransaction;
use futures::channel::mpsc::Sender;
use itertools::Either;
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
    time::Duration,
};
use tokio::{sync::oneshot, time::timeout};

pub trait TPayloadManager: Send + Sync {
    fn prefetch_payload_data(&self, payload: &Payload, timestamp: u64);
}

/// Responsible to extract the transactions out of the payload and notify QuorumStore about commits.
/// If QuorumStore is enabled, has to ask BatchReader for the transaction behind the proofs of availability in the payload.
pub enum PayloadManager {
    DirectMempool,
    InQuorumStore(
        Arc<dyn BatchReader>,
        Sender<CoordinatorCommand>,
        Option<Arc<ConsensusPublisher>>,
    ),
    ConsensusObserver(
        Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
        Option<Arc<ConsensusPublisher>>,
    ),
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
            PayloadManager::DirectMempool | PayloadManager::ConsensusObserver(_, _) => {},
            PayloadManager::InQuorumStore(batch_reader, coordinator_tx, _) => {
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
                if proof_with_status.status.lock().is_some() {
                    return;
                }
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
            PayloadManager::DirectMempool | PayloadManager::ConsensusObserver(_, _) => {},
            PayloadManager::InQuorumStore(batch_reader, _, _) => match payload {
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
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>)> {
        let payload = match block.payload() {
            Some(p) => p,
            None => return Ok((Vec::new(), None)),
        };

        if let PayloadManager::ConsensusObserver(txns_pool, consensus_publisher) = self {
            return get_transactions_for_observer(block, payload, txns_pool, consensus_publisher)
                .await;
        }

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

        let (transactions, limit, proof_with_data, inline_batches) = match (self, payload) {
            (PayloadManager::DirectMempool, Payload::DirectMempool(txns)) => {
                return Ok((txns.clone(), None))
            },
            (
                PayloadManager::InQuorumStore(batch_reader, _, _),
                Payload::InQuorumStore(proof_with_data),
            ) => (
                process_payload(proof_with_data, batch_reader.clone(), block).await?,
                None,
                proof_with_data.clone(),
                vec![],
            ),
            (
                PayloadManager::InQuorumStore(batch_reader, _, _),
                Payload::InQuorumStoreWithLimit(proof_with_data),
            ) => (
                process_payload(
                    &proof_with_data.proof_with_data,
                    batch_reader.clone(),
                    block,
                )
                .await?,
                proof_with_data.max_txns_to_execute,
                proof_with_data.proof_with_data.clone(),
                vec![],
            ),
            (
                PayloadManager::InQuorumStore(batch_reader, _, _),
                Payload::QuorumStoreInlineHybrid(
                    inline_batches,
                    proof_with_data,
                    max_txns_to_execute,
                ),
            ) => (
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
                proof_with_data.clone(),
                inline_batches
                    .iter()
                    .map(|(batch_info, _)| batch_info.clone())
                    .collect(),
            ),
            (_, _) => unreachable!(
                "Wrong payload {} epoch {}, round {}, id {}",
                payload,
                block.block_data().epoch(),
                block.block_data().round(),
                block.id()
            ),
        };

        if let PayloadManager::InQuorumStore(_, _, Some(consensus_publisher)) = self {
            let transaction_payload = BlockTransactionPayload::new(
                transactions.clone(),
                limit,
                proof_with_data,
                inline_batches,
            );
            let message = ConsensusObserverMessage::new_block_payload_message(
                block.gen_block_info(HashValue::zero(), 0, None),
                transaction_payload,
            );
            consensus_publisher.publish_message(message).await;
        }

        Ok((transactions, limit))
    }
}

async fn get_transactions_for_observer(
    block: &Block,
    payload: &Payload,
    txns_pool: &Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
    consensus_publisher: &Option<Arc<ConsensusPublisher>>,
) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>)> {
    // If the data is already available, return it, otherwise wait for it.
    // It's important to make sure this doesn't race with the payload insertion part.
    let result = match txns_pool.lock().entry((block.epoch(), block.round())) {
        Entry::Occupied(mut value) => match value.get_mut() {
            BlockPayloadStatus::AvailableAndVerified(data) => Either::Left(data.clone()),
            BlockPayloadStatus::AvailableAndUnverified(_, payload_sender) => {
                let (new_payload_sender, new_payload_receiver) = oneshot::channel();
                *payload_sender = Some(new_payload_sender);
                Either::Right(new_payload_receiver)
            },
            BlockPayloadStatus::Requested(payload_sender) => {
                let (new_payload_sender, new_payload_receiver) = oneshot::channel();
                *payload_sender = new_payload_sender;
                Either::Right(new_payload_receiver)
            },
        },
        Entry::Vacant(entry) => {
            let (payload_sender, payload_receiver) = oneshot::channel();
            entry.insert(BlockPayloadStatus::Requested(payload_sender));
            Either::Right(payload_receiver)
        },
    };

    let block_transaction_payload = match result {
        Either::Left(data) => data.transaction_payload,
        Either::Right(rx) => timeout(Duration::from_millis(300), rx)
            .await
            .map_err(|_| ExecutorError::CouldNotGetData)?
            .map_err(|_| ExecutorError::CouldNotGetData)?,
    };

    // Verify the payload and inline batches before returning the data. The
    // batch digests and transactions will have already been verified by the
    // consensus observer on message receipt.
    match payload {
        Payload::DirectMempool(_) => {
            return Err(ExecutorError::InternalError {
                error: "DirectMempool payloads should not be sent to the consensus observer!"
                    .to_string(),
            });
        },
        Payload::InQuorumStore(proof_with_data) => {
            // Verify the batches in the requested block
            verify_batches_in_block(&proof_with_data.proofs, &block_transaction_payload)?;
        },
        Payload::InQuorumStoreWithLimit(proof_with_data) => {
            // Verify the batches in the requested block
            verify_batches_in_block(
                &proof_with_data.proof_with_data.proofs,
                &block_transaction_payload,
            )?;

            // Verify the transaction limit
            verify_transaction_limit(
                proof_with_data.max_txns_to_execute,
                &block_transaction_payload,
            )?;
        },
        Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, max_txns_to_execute) => {
            // Verify the batches in the requested block
            verify_batches_in_block(&proof_with_data.proofs, &block_transaction_payload)?;

            // Verify the inline batches
            verify_inline_batches_in_block(inline_batches, &block_transaction_payload)?;

            // Verify the transaction limit
            verify_transaction_limit(*max_txns_to_execute, &block_transaction_payload)?;
        },
    }

    if let Some(consensus_publisher) = consensus_publisher {
        let message = ConsensusObserverMessage::new_block_payload_message(
            block.gen_block_info(HashValue::zero(), 0, None),
            block_transaction_payload.clone(),
        );
        consensus_publisher.publish_message(message).await;
    }

    Ok((
        block_transaction_payload.transactions,
        block_transaction_payload.limit,
    ))
}

fn verify_batches_in_block(
    verified_proofs: &[ProofOfStore],
    block_transaction_payload: &BlockTransactionPayload,
) -> ExecutorResult<()> {
    let verified_batches: Vec<&BatchInfo> =
        verified_proofs.iter().map(|proof| proof.info()).collect();
    let found_batches: Vec<&BatchInfo> = block_transaction_payload
        .proof_with_data
        .proofs
        .iter()
        .map(|proof| proof.info())
        .collect();

    if verified_batches != found_batches {
        Err(ExecutorError::InternalError {
            error: format!(
                "Expected batches {:?} but found {:?}!",
                verified_batches, found_batches
            ),
        })
    } else {
        Ok(())
    }
}

fn verify_inline_batches_in_block(
    verified_inline_batches: &[(BatchInfo, Vec<SignedTransaction>)],
    block_transaction_payload: &BlockTransactionPayload,
) -> ExecutorResult<()> {
    let verified_batches: Vec<BatchInfo> = verified_inline_batches
        .iter()
        .map(|(batch_info, _)| batch_info.clone())
        .collect();
    let found_inline_batches = &block_transaction_payload.inline_batches;

    if verified_batches != *found_inline_batches {
        Err(ExecutorError::InternalError {
            error: format!(
                "Expected inline batches {:?} but found {:?}",
                verified_batches, found_inline_batches
            ),
        })
    } else {
        Ok(())
    }
}

fn verify_transaction_limit(
    max_txns_to_execute: Option<u64>,
    block_transaction_payload: &BlockTransactionPayload,
) -> ExecutorResult<()> {
    if max_txns_to_execute != block_transaction_payload.limit {
        Err(ExecutorError::InternalError {
            error: format!(
                "Expected transaction limit {:?} but found {:?}",
                max_txns_to_execute, block_transaction_payload.limit
            ),
        })
    } else {
        Ok(())
    }
}
