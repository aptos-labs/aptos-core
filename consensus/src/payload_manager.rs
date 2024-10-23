// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        network::observer_message::{BlockTransactionPayload, ConsensusObserverMessage},
        observer::payload_store::BlockPayloadStatus,
        publisher::consensus_publisher::ConsensusPublisher,
    },
    counters,
    quorum_store::{batch_store::BatchReader, quorum_store_coordinator::CoordinatorCommand},
};
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{DataStatus, Payload, ProofWithData, Round},
    payload::{BatchPointer, DataFetchFut, TDataInfo},
    pipelined_block::PipelinedBlock,
    proof_of_store::BatchInfo,
};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    ExecutorError::{DataNotFound, InternalError},
    *,
};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{transaction::SignedTransaction, PeerId};
use async_trait::async_trait;
use futures::{channel::mpsc::Sender, FutureExt};
use itertools::Itertools;
use std::{
    collections::{btree_map::Entry, BTreeMap, HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};
use tokio::sync::oneshot;

/// A trait that defines the interface for a payload manager. The payload manager is responsible for
/// resolving the transactions in a block's payload.
#[async_trait]
pub trait TPayloadManager: Send + Sync {
    /// Notify the payload manager that a block has been committed. This indicates that the
    /// transactions in the block's payload are no longer required for consensus.
    fn notify_commit(&self, block_timestamp: u64, block: Option<PipelinedBlock>);

    fn notify_ordered(&self, block: PipelinedBlock);

    /// Prefetch the data for a payload. This is used to ensure that the data for a payload is
    /// available when block is executed.
    fn prefetch_payload_data(&self, payload: &Payload, timestamp: u64);

    /// Check if the transactions corresponding are available. This is specific to payload
    /// manager implementations. For optimistic quorum store, we only check if optimistic
    /// batches are available locally.
    fn check_payload_availability(&self, block: &Block) -> Result<(), BitVec>;

    /// Get the transactions in a block's payload. This function returns a vector of transactions.
    async fn get_transactions(
        &self,
        block: &Block,
    ) -> ExecutorResult<(
        Vec<(Arc<Vec<SignedTransaction>>, u64)>,
        Option<u64>,
        Option<u64>,
    )>;
}

/// A payload manager that directly returns the transactions in a block's payload.
pub struct DirectMempoolPayloadManager {}

impl DirectMempoolPayloadManager {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl TPayloadManager for DirectMempoolPayloadManager {
    fn notify_commit(&self, _block_timestamp: u64, _block: Option<PipelinedBlock>) {}

    fn notify_ordered(&self, _block: PipelinedBlock) {}

    fn prefetch_payload_data(&self, _payload: &Payload, _timestamp: u64) {}

    fn check_payload_availability(&self, _block: &Block) -> Result<(), BitVec> {
        Ok(())
    }

    async fn get_transactions(
        &self,
        block: &Block,
    ) -> ExecutorResult<(
        Vec<(Arc<Vec<SignedTransaction>>, u64)>,
        Option<u64>,
        Option<u64>,
    )> {
        let Some(payload) = block.payload() else {
            return Ok((Vec::new(), None, None));
        };

        match payload {
            Payload::DirectMempool(txns) => Ok((vec![(Arc::new(txns.clone()), 0)], None, None)),
            _ => unreachable!(
                "DirectMempoolPayloadManager: Unacceptable payload type {}. Epoch: {}, Round: {}, Block: {}",
                payload,
                block.block_data().epoch(),
                block.block_data().round(),
                block.id()
            ),
        }
    }
}

/// A payload manager that resolves the transactions in a block's payload from the quorum store.
pub struct QuorumStorePayloadManager {
    batch_reader: Arc<dyn BatchReader>,
    coordinator_tx: Sender<CoordinatorCommand>,
    maybe_consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ordered_authors: Vec<PeerId>,
    address_to_validator_index: HashMap<PeerId, usize>,
}

impl QuorumStorePayloadManager {
    pub fn new(
        batch_reader: Arc<dyn BatchReader>,
        coordinator_tx: Sender<CoordinatorCommand>,
        maybe_consensus_publisher: Option<Arc<ConsensusPublisher>>,
        ordered_authors: Vec<PeerId>,
        address_to_validator_index: HashMap<PeerId, usize>,
    ) -> Self {
        Self {
            batch_reader,
            coordinator_tx,
            maybe_consensus_publisher,
            ordered_authors,
            address_to_validator_index,
        }
    }

    fn request_transactions(
        batches: Vec<(BatchInfo, Vec<PeerId>)>,
        block_timestamp: u64,
        batch_reader: Arc<dyn BatchReader>,
    ) -> Vec<(
        HashValue,
        u64,
        oneshot::Receiver<ExecutorResult<Arc<Vec<SignedTransaction>>>>,
    )> {
        let mut receivers = Vec::new();
        for (batch_info, responders) in batches {
            trace!(
                "QSE: requesting batch {:?}, time = {}",
                batch_info,
                block_timestamp
            );
            if block_timestamp <= batch_info.expiration() {
                receivers.push((
                    *batch_info.digest(),
                    batch_info.gas_bucket_start(),
                    batch_reader.get_batch(
                        *batch_info.digest(),
                        batch_info.expiration(),
                        responders,
                    ),
                ));
            } else {
                debug!("QSE: skipped expired batch {}", batch_info.digest());
            }
        }
        receivers
    }

    fn batches_in_block(block: &Block) -> Vec<BatchInfo> {
        let mut batches = vec![];
        for payload in block.payload().iter() {
            match payload {
                Payload::DirectMempool(_) => {
                    unreachable!("InQuorumStore should be used");
                },
                Payload::InQuorumStore(proof_with_status) => {
                    for proof in proof_with_status.proofs.iter() {
                        batches.push(proof.info().clone());
                    }
                },
                Payload::InQuorumStoreWithLimit(proof_with_status) => {
                    for proof in proof_with_status.proof_with_data.proofs.iter() {
                        batches.push(proof.info().clone());
                    }
                },
                Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _, _) => {
                    for (batch_info, _) in inline_batches.iter() {
                        batches.push(batch_info.clone());
                    }
                    for proof in proof_with_data.proofs.iter() {
                        batches.push(proof.info().clone());
                    }
                },
                Payload::OptQuorumStore(opt_quorum_store_payload) => {
                    // TODO: how to avoid the clone?
                    batches = opt_quorum_store_payload
                        .clone()
                        .into_inner()
                        .get_all_batch_infos();
                },
            }
        }
        batches
    }

    fn batches_removed_from_window(block: &PipelinedBlock) -> Vec<BatchInfo> {
        let mut batches_removed = HashSet::new();
        if let Some(block_removed) = block
            .block_window()
            .blocks()
            .iter()
            .chain(std::iter::once(block.block()))
            .next()
        {
            for batch in Self::batches_in_block(block_removed) {
                batches_removed.insert(batch);
            }
        }
        block
            .block_window()
            .blocks()
            .iter()
            .chain(std::iter::once(block.block()))
            .skip(1)
            .for_each(|block| {
                for batch in Self::batches_in_block(block) {
                    batches_removed.remove(&batch);
                }
            });
        batches_removed.into_iter().collect()
    }
}

#[async_trait]
impl TPayloadManager for QuorumStorePayloadManager {
    fn notify_commit(&self, block_timestamp: u64, block: Option<PipelinedBlock>) {
        self.batch_reader
            .update_certified_timestamp(block_timestamp);

        let batches_to_remove =
            block.map_or(vec![], |block| Self::batches_removed_from_window(&block));
        info!(
            "batches_to_remove: {}",
            batches_to_remove
                .iter()
                .map(|b| format!("{}", b))
                .join(", ")
        );

        let mut tx = self.coordinator_tx.clone();

        if let Err(e) = tx.try_send(CoordinatorCommand::CommitNotification(
            block_timestamp,
            batches_to_remove,
        )) {
            warn!(
                "CommitNotification failed. Is the epoch shutting down? error: {}",
                e
            );
        }
    }

    fn notify_ordered(&self, block: PipelinedBlock) {
        let mut tx = self.coordinator_tx.clone();
        if let Err(e) = tx.try_send(CoordinatorCommand::OrderedNotification(block)) {
            warn!(
                "BlockOrdered notification failed. Is the epoch shutting down? error: {}",
                e
            );
        }
    }

    fn prefetch_payload_data(&self, payload: &Payload, timestamp: u64) {
        // This is deprecated.
        // TODO(ibalajiarun): Remove this after migrating to OptQuorumStore type
        let request_txns_and_update_status =
            move |proof_with_status: &ProofWithData, batch_reader: Arc<dyn BatchReader>| {
                if proof_with_status.status.lock().is_some() {
                    return;
                }
                let receivers = Self::request_transactions(
                    proof_with_status
                        .proofs
                        .iter()
                        .map(|proof| {
                            (
                                proof.info().clone(),
                                proof.shuffled_signers(&self.ordered_authors),
                            )
                        })
                        .collect(),
                    timestamp,
                    batch_reader,
                );
                proof_with_status
                    .status
                    .lock()
                    .replace(DataStatus::Requested(receivers));
            };

        fn prefetch_helper<T: TDataInfo>(
            data_pointer: &BatchPointer<T>,
            batch_reader: Arc<dyn BatchReader>,
            timestamp: u64,
            ordered_authors: &[PeerId],
        ) {
            let mut data_fut = data_pointer.data_fut.lock();
            if data_fut.is_some() {
                return;
            }

            let batches_and_responders = data_pointer
                .batch_summary
                .iter()
                .map(|proof| {
                    let signers = proof.signers(ordered_authors);
                    // TODO(ibalajiarun): Add block author to signers
                    (proof.info().clone(), signers)
                })
                .collect();
            let fut =
                request_txns_from_quorum_store(batches_and_responders, timestamp, batch_reader)
                    .boxed()
                    .shared();
            *data_fut = Some(DataFetchFut { fut, iteration: 0 })
        }

        match payload {
            Payload::InQuorumStore(proof_with_status) => {
                request_txns_and_update_status(proof_with_status, self.batch_reader.clone());
            },
            Payload::InQuorumStoreWithLimit(proof_with_data) => {
                request_txns_and_update_status(
                    &proof_with_data.proof_with_data,
                    self.batch_reader.clone(),
                );
            },
            Payload::QuorumStoreInlineHybrid(_, proof_with_data, _, _) => {
                request_txns_and_update_status(proof_with_data, self.batch_reader.clone());
            },
            Payload::DirectMempool(_) => {
                unreachable!()
            },
            Payload::OptQuorumStore(opt_qs_payload) => {
                prefetch_helper(
                    opt_qs_payload.opt_batches(),
                    self.batch_reader.clone(),
                    timestamp,
                    &self.ordered_authors,
                );
                prefetch_helper(
                    opt_qs_payload.proof_with_data(),
                    self.batch_reader.clone(),
                    timestamp,
                    &self.ordered_authors,
                )
            },
        };
    }

    fn check_payload_availability(&self, block: &Block) -> Result<(), BitVec> {
        let Some(payload) = block.payload() else {
            return Ok(());
        };

        match payload {
            Payload::DirectMempool(_) => {
                unreachable!("QuorumStore doesn't support DirectMempool payload")
            },
            Payload::InQuorumStore(_) => Ok(()),
            Payload::InQuorumStoreWithLimit(_) => Ok(()),
            Payload::QuorumStoreInlineHybrid(inline_batches, proofs, _, _) => {
                fn update_availability_metrics<'a>(
                    batch_reader: &Arc<dyn BatchReader>,
                    is_proof_label: &str,
                    batch_infos: impl Iterator<Item = &'a BatchInfo>,
                ) {
                    for (author, chunk) in &batch_infos.chunk_by(|info| info.author()) {
                        let (available_count, missing_count) = chunk
                            .map(|info| batch_reader.exists(info.digest()))
                            .fold((0, 0), |(available_count, missing_count), item| {
                                if item.is_some() {
                                    (available_count + 1, missing_count)
                                } else {
                                    (available_count, missing_count + 1)
                                }
                            });
                        counters::CONSENSUS_PROPOSAL_PAYLOAD_BATCH_AVAILABILITY_IN_QS
                            .with_label_values(&[
                                &author.to_hex_literal(),
                                is_proof_label,
                                "available",
                            ])
                            .inc_by(available_count as u64);
                        counters::CONSENSUS_PROPOSAL_PAYLOAD_BATCH_AVAILABILITY_IN_QS
                            .with_label_values(&[
                                &author.to_hex_literal(),
                                is_proof_label,
                                "missing",
                            ])
                            .inc_by(missing_count as u64);
                    }
                }

                update_availability_metrics(
                    &self.batch_reader,
                    "false",
                    inline_batches.iter().map(|(batch_info, _)| batch_info),
                );
                update_availability_metrics(
                    &self.batch_reader,
                    "true",
                    proofs.proofs.iter().map(|proof| proof.info()),
                );

                // The payload is considered available because it contains only proofs that guarantee network availabiliy
                // or inlined transactions.
                Ok(())
            },
            Payload::OptQuorumStore(opt_qs_payload) => {
                let mut missing_authors = BitVec::with_num_bits(self.ordered_authors.len() as u16);
                for batch in opt_qs_payload.opt_batches().deref() {
                    if self.batch_reader.exists(batch.digest()).is_none() {
                        let index = *self
                            .address_to_validator_index
                            .get(&batch.author())
                            .expect("Payload author should have been verified");
                        missing_authors.set(index as u16);
                    }
                }
                if missing_authors.all_zeros() {
                    Ok(())
                } else {
                    Err(missing_authors)
                }
            },
        }
    }

    async fn get_transactions(
        &self,
        block: &Block,
    ) -> ExecutorResult<(
        Vec<(Arc<Vec<SignedTransaction>>, u64)>,
        Option<u64>,
        Option<u64>,
    )> {
        info!(
            "get_transactions for block ({}, {}) started.",
            block.epoch(),
            block.round()
        );
        let Some(payload) = block.payload() else {
            info!(
                "get_transactions for block ({}, {}) finished (empty).",
                block.epoch(),
                block.round()
            );
            return Ok((Vec::new(), None, None));
        };

        let transaction_payload = match payload {
            Payload::InQuorumStore(proof_with_data) => {
                let transactions = process_payload(
                    proof_with_data,
                    self.batch_reader.clone(),
                    block,
                    &self.ordered_authors,
                )
                .await?;
                BlockTransactionPayload::new_in_quorum_store(
                    transactions,
                    proof_with_data.proofs.clone(),
                )
            },
            Payload::InQuorumStoreWithLimit(proof_with_data) => {
                let transactions = process_payload(
                    &proof_with_data.proof_with_data,
                    self.batch_reader.clone(),
                    block,
                    &self.ordered_authors,
                )
                .await?;
                BlockTransactionPayload::new_in_quorum_store_with_limit(
                    transactions,
                    proof_with_data.proof_with_data.proofs.clone(),
                    proof_with_data.max_txns_to_execute,
                    proof_with_data.block_gas_limit,
                )
            },
            Payload::QuorumStoreInlineHybrid(
                inline_batches,
                proof_with_data,
                max_txns_to_execute,
                block_gas_limit,
            ) => {
                let all_transactions = {
                    let mut all_txns = process_payload(
                        proof_with_data,
                        self.batch_reader.clone(),
                        block,
                        &self.ordered_authors,
                    )
                    .await?;
                    all_txns.append(
                        &mut inline_batches
                            .iter()
                            // TODO: Can clone be avoided here?
                            .map(|(batch_info, txns)| (txns.clone(), batch_info.gas_bucket_start()))
                            .collect(),
                    );
                    all_txns
                };
                let inline_batches = inline_batches
                    .iter()
                    .map(|(batch_info, _)| batch_info.clone())
                    .collect();
                BlockTransactionPayload::new_quorum_store_inline_hybrid(
                    all_transactions,
                    proof_with_data.proofs.clone(),
                    *max_txns_to_execute,
                    *block_gas_limit,
                    inline_batches,
                )
            },
            Payload::OptQuorumStore(opt_qs_payload) => {
                let opt_batch_txns = process_payload_helper(
                    opt_qs_payload.opt_batches(),
                    self.batch_reader.clone(),
                    block,
                    &self.ordered_authors,
                )
                .await?;
                let proof_batch_txns = process_payload_helper(
                    opt_qs_payload.proof_with_data(),
                    self.batch_reader.clone(),
                    block,
                    &self.ordered_authors,
                )
                .await?;
                // TODO: this is a complete hack, need to add real support for OptQuorumStore
                let inline_batch_txns = opt_qs_payload.inline_batches().transactions();
                let all_txns = [proof_batch_txns, opt_batch_txns, inline_batch_txns].concat();
                BlockTransactionPayload::new_opt_quorum_store(
                    all_txns,
                    opt_qs_payload.proof_with_data().deref().clone(),
                    opt_qs_payload.max_txns_to_execute(),
                    opt_qs_payload.block_gas_limit(),
                    [
                        opt_qs_payload.opt_batches().deref().clone(),
                        opt_qs_payload.inline_batches().batch_infos(),
                    ]
                    .concat(),
                )
            },
            _ => unreachable!(
                "Wrong payload {} epoch {}, round {}, id {}",
                payload,
                block.block_data().epoch(),
                block.block_data().round(),
                block.id()
            ),
        };

        if let Some(consensus_publisher) = &self.maybe_consensus_publisher {
            let message = ConsensusObserverMessage::new_block_payload_message(
                block.gen_block_info(HashValue::zero(), 0, None),
                transaction_payload.clone(),
            );
            consensus_publisher.publish_message(message);
        }

        info!(
            "get_transactions for block ({}, {}) finished.",
            block.epoch(),
            block.round()
        );

        Ok((
            transaction_payload.transactions(),
            transaction_payload.transaction_limit(),
            transaction_payload.block_gas_limit(),
        ))
    }
}

/// Returns the transactions for the consensus observer payload manager
async fn get_transactions_for_observer(
    block: &Block,
    block_payloads: &Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
    consensus_publisher: &Option<Arc<ConsensusPublisher>>,
) -> ExecutorResult<(
    Vec<(Arc<Vec<SignedTransaction>>, u64)>,
    Option<u64>,
    Option<u64>,
)> {
    // The data should already be available (as consensus observer will only ever
    // forward a block to the executor once the data has been received and verified).
    let block_payload = match block_payloads.lock().entry((block.epoch(), block.round())) {
        Entry::Occupied(mut value) => match value.get_mut() {
            BlockPayloadStatus::AvailableAndVerified(block_payload) => block_payload.clone(),
            BlockPayloadStatus::AvailableAndUnverified(_) => {
                // This shouldn't happen (the payload should already be verified)
                let error = format!(
                    "Payload data for block epoch {}, round {} is unverified!",
                    block.epoch(),
                    block.round()
                );
                return Err(InternalError { error });
            },
        },
        Entry::Vacant(_) => {
            // This shouldn't happen (the payload should already be present)
            let error = format!(
                "Missing payload data for block epoch {}, round {}!",
                block.epoch(),
                block.round()
            );
            return Err(InternalError { error });
        },
    };

    // If the payload is valid, publish it to any downstream observers
    let transaction_payload = block_payload.transaction_payload();
    if let Some(consensus_publisher) = consensus_publisher {
        let message = ConsensusObserverMessage::new_block_payload_message(
            block.gen_block_info(HashValue::zero(), 0, None),
            transaction_payload.clone(),
        );
        consensus_publisher.publish_message(message);
    }

    // Return the transactions and the transaction limit
    Ok((
        transaction_payload.transactions(),
        transaction_payload.transaction_limit(),
        transaction_payload.block_gas_limit(),
    ))
}

async fn request_txns_from_quorum_store(
    batches_and_responders: Vec<(BatchInfo, Vec<PeerId>)>,
    timestamp: u64,
    batch_reader: Arc<dyn BatchReader>,
) -> ExecutorResult<Vec<(Arc<Vec<SignedTransaction>>, u64)>> {
    let mut vec_ret = Vec::new();
    let receivers = QuorumStorePayloadManager::request_transactions(
        batches_and_responders,
        timestamp,
        batch_reader,
    );
    for (digest, gas_bucket_start, rx) in receivers {
        match rx.await {
            Err(e) => {
                // We probably advanced epoch already.
                warn!(
                    "Oneshot channel to get a batch was dropped with error {:?}",
                    e
                );
                return Err(DataNotFound(digest));
            },
            Ok(Ok(data)) => {
                vec_ret.push((data, gas_bucket_start));
            },
            Ok(Err(e)) => {
                return Err(e);
            },
        }
    }
    Ok(vec_ret)
}

async fn process_payload_helper<T: TDataInfo>(
    data_ptr: &BatchPointer<T>,
    batch_reader: Arc<dyn BatchReader>,
    block: &Block,
    ordered_authors: &[PeerId],
) -> ExecutorResult<Vec<(Arc<Vec<SignedTransaction>>, u64)>> {
    let (iteration, fut) = {
        let data_fut_guard = data_ptr.data_fut.lock();
        let data_fut = data_fut_guard.as_ref().expect("must be initialized");
        (data_fut.iteration, data_fut.fut.clone())
    };

    let result = fut.await;
    // If error, reschedule before returning the result
    if result.is_err() {
        let mut data_fut_guard = data_ptr.data_fut.lock();
        let data_fut = data_fut_guard.as_mut().expect("must be initialized");
        // Protection against race, check the iteration number before rescheduling.
        if data_fut.iteration == iteration {
            let batches_and_responders = data_ptr
                .batch_summary
                .iter()
                .map(|proof| {
                    let mut signers = proof.signers(ordered_authors);
                    if let Some(author) = block.author() {
                        signers.push(author);
                    }
                    (proof.info().clone(), signers)
                })
                .collect();
            data_fut.fut = request_txns_from_quorum_store(
                batches_and_responders,
                block.timestamp_usecs(),
                batch_reader,
            )
            .boxed()
            .shared();
            data_fut.iteration = iteration + 1;
        }
    }
    result
}

/// This is deprecated. Use `process_payload_helper` instead after migrating to
/// OptQuorumStore payload
async fn process_payload(
    proof_with_data: &ProofWithData,
    batch_reader: Arc<dyn BatchReader>,
    block: &Block,
    ordered_authors: &[PeerId],
    // TODO: replace this Vec<(Arc<Vec<>>, u64>> with a struct BatchedTransactions
) -> ExecutorResult<Vec<(Arc<Vec<SignedTransaction>>, u64)>> {
    let status = proof_with_data.status.lock().take();
    match status.expect("Should have been updated before.") {
        DataStatus::Cached(data) => {
            info!(
                "get_transactions block ({},{}) data is cached.",
                block.epoch(),
                block.round()
            );
            counters::QUORUM_BATCH_READY_COUNT.inc();
            proof_with_data
                .status
                .lock()
                .replace(DataStatus::Cached(data.clone()));
            Ok(data)
        },
        DataStatus::Requested(receivers) => {
            let _timer = counters::BATCH_WAIT_DURATION.start_timer();
            info!(
                "get_transactions block ({},{}) data is being requested.",
                block.epoch(),
                block.round()
            );
            let mut vec_ret = Vec::new();
            if !receivers.is_empty() {
                debug!(
                    "QSE: waiting for data on {} receivers, block_round {}",
                    receivers.len(),
                    block.round()
                );
            }
            for (digest, gas_bucket_start, rx) in receivers {
                match rx.await {
                    Err(e) => {
                        // We probably advanced epoch already.
                        warn!(
                            "Oneshot channel to get a batch was dropped with error {:?}",
                            e
                        );
                        let new_receivers = QuorumStorePayloadManager::request_transactions(
                            proof_with_data
                                .proofs
                                .iter()
                                .map(|proof| {
                                    (
                                        proof.info().clone(),
                                        proof.shuffled_signers(ordered_authors),
                                    )
                                })
                                .collect(),
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
                        vec_ret.push((data, gas_bucket_start));
                    },
                    Ok(Err(e)) => {
                        let new_receivers = QuorumStorePayloadManager::request_transactions(
                            proof_with_data
                                .proofs
                                .iter()
                                .map(|proof| {
                                    (
                                        proof.info().clone(),
                                        proof.shuffled_signers(ordered_authors),
                                    )
                                })
                                .collect(),
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
            // execution asks for the data twice, so data is cached here for the second time.
            proof_with_data
                .status
                .lock()
                .replace(DataStatus::Cached(vec_ret.clone()));
            Ok(vec_ret)
        },
    }
}

pub struct ConsensusObserverPayloadManager {
    txns_pool: Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
}

impl ConsensusObserverPayloadManager {
    pub fn new(
        txns_pool: Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ) -> Self {
        Self {
            txns_pool,
            consensus_publisher,
        }
    }
}

#[async_trait]
impl TPayloadManager for ConsensusObserverPayloadManager {
    fn notify_commit(&self, _block_timestamp: u64, _block: Option<PipelinedBlock>) {
        // noop
    }

    fn notify_ordered(&self, _block: PipelinedBlock) {
        // noop
    }

    fn prefetch_payload_data(&self, _payload: &Payload, _timestamp: u64) {
        // noop
    }

    fn check_payload_availability(&self, _block: &Block) -> Result<(), BitVec> {
        unreachable!("this method isn't used in ConsensusObserver")
    }

    async fn get_transactions(
        &self,
        block: &Block,
    ) -> ExecutorResult<(
        Vec<(Arc<Vec<SignedTransaction>>, u64)>,
        Option<u64>,
        Option<u64>,
    )> {
        return get_transactions_for_observer(block, &self.txns_pool, &self.consensus_publisher)
            .await;
    }
}
