// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        network::observer_message::{BlockTransactionPayload, ConsensusObserverMessage},
        publisher::consensus_publisher::ConsensusPublisher,
    },
    counters,
    payload_manager::TPayloadManager,
    quorum_store::{batch_store::BatchReader, quorum_store_coordinator::CoordinatorCommand},
};
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload, ProofWithData},
    payload::{BatchPointer, TDataInfo},
    proof_of_store::BatchInfo,
};
use aptos_crypto::HashValue;
use aptos_executor_types::*;
use aptos_logger::prelude::*;
use aptos_types::{transaction::SignedTransaction, PeerId};
use async_trait::async_trait;
use futures::{channel::mpsc::Sender, future::Shared};
use itertools::Itertools;
use std::{collections::HashMap, future::Future, ops::Deref, pin::Pin, sync::Arc};

pub trait TQuorumStoreCommitNotifier: Send + Sync {
    fn notify(&self, block_timestamp: u64, batches: Vec<BatchInfo>);
}

pub struct QuorumStoreCommitNotifier {
    coordinator_tx: Sender<CoordinatorCommand>,
}

impl QuorumStoreCommitNotifier {
    pub fn new(coordinator_tx: Sender<CoordinatorCommand>) -> Self {
        Self { coordinator_tx }
    }
}

impl TQuorumStoreCommitNotifier for QuorumStoreCommitNotifier {
    fn notify(&self, block_timestamp: u64, batches: Vec<BatchInfo>) {
        let mut tx = self.coordinator_tx.clone();

        if let Err(e) = tx.try_send(CoordinatorCommand::CommitNotification(
            block_timestamp,
            batches,
        )) {
            warn!(
                "CommitNotification failed. Is the epoch shutting down? error: {}",
                e
            );
        }
    }
}

/// A payload manager that resolves the transactions in a block's payload from the quorum store.
pub struct QuorumStorePayloadManager {
    batch_reader: Arc<dyn BatchReader>,
    commit_notifier: Box<dyn TQuorumStoreCommitNotifier>,
    maybe_consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ordered_authors: Vec<PeerId>,
    address_to_validator_index: HashMap<PeerId, usize>,
}

impl QuorumStorePayloadManager {
    pub fn new(
        batch_reader: Arc<dyn BatchReader>,
        commit_notifier: Box<dyn TQuorumStoreCommitNotifier>,
        maybe_consensus_publisher: Option<Arc<ConsensusPublisher>>,
        ordered_authors: Vec<PeerId>,
        address_to_validator_index: HashMap<PeerId, usize>,
    ) -> Self {
        Self {
            batch_reader,
            commit_notifier,
            maybe_consensus_publisher,
            ordered_authors,
            address_to_validator_index,
        }
    }

    fn request_transactions(
        batches: Vec<(BatchInfo, Vec<PeerId>)>,
        block_timestamp: u64,
        batch_reader: Arc<dyn BatchReader>,
    ) -> Vec<Shared<Pin<Box<dyn Future<Output = ExecutorResult<Vec<SignedTransaction>>> + Send>>>>
    {
        let mut futures = Vec::new();
        for (batch_info, responders) in batches {
            trace!(
                "QSE: requesting batch {:?}, time = {}",
                batch_info,
                block_timestamp
            );
            if block_timestamp <= batch_info.expiration() {
                futures.push(batch_reader.get_batch(batch_info, responders.clone()));
            } else {
                debug!("QSE: skipped expired batch {}", batch_info.digest());
            }
        }
        futures
    }

    async fn request_and_wait_transactions(
        batches: Vec<(BatchInfo, Vec<PeerId>)>,
        block_timestamp: u64,
        batch_reader: Arc<dyn BatchReader>,
    ) -> ExecutorResult<Vec<SignedTransaction>> {
        let futures = Self::request_transactions(batches, block_timestamp, batch_reader);
        let mut all_txns = Vec::new();
        for result in futures::future::join_all(futures).await {
            all_txns.append(&mut result?);
        }
        Ok(all_txns)
    }
}

#[async_trait]
impl TPayloadManager for QuorumStorePayloadManager {
    fn notify_commit(&self, block_timestamp: u64, payloads: Vec<Payload>) {
        self.batch_reader
            .update_certified_timestamp(block_timestamp);

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
                Payload::OptQuorumStore(opt_quorum_store_payload) => {
                    opt_quorum_store_payload.into_inner().get_all_batch_infos()
                },
            })
            .collect();

        self.commit_notifier.notify(block_timestamp, batches);
    }

    fn prefetch_payload_data(&self, payload: &Payload, author: Author, timestamp: u64) {
        // This is deprecated.
        // TODO(ibalajiarun): Remove this after migrating to OptQuorumStore type
        let request_txns_and_update_status =
            move |proof_with_status: &ProofWithData, batch_reader: Arc<dyn BatchReader>| {
                Self::request_transactions(
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
            };

        fn prefetch_helper<T: TDataInfo>(
            data_pointer: &BatchPointer<T>,
            batch_reader: Arc<dyn BatchReader>,
            author: Option<Author>,
            timestamp: u64,
            ordered_authors: &[PeerId],
        ) {
            let batches_and_responders = data_pointer
                .batch_summary
                .iter()
                .map(|data_info| {
                    let mut signers = data_info.signers(ordered_authors);
                    if let Some(author) = author {
                        signers.push(author);
                    }
                    (data_info.info().clone(), signers)
                })
                .collect();
            QuorumStorePayloadManager::request_transactions(
                batches_and_responders,
                timestamp,
                batch_reader,
            );
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
            Payload::QuorumStoreInlineHybrid(_, proof_with_data, _) => {
                request_txns_and_update_status(proof_with_data, self.batch_reader.clone());
            },
            Payload::DirectMempool(_) => {
                unreachable!()
            },
            Payload::OptQuorumStore(opt_qs_payload) => {
                prefetch_helper(
                    opt_qs_payload.opt_batches(),
                    self.batch_reader.clone(),
                    Some(author),
                    timestamp,
                    &self.ordered_authors,
                );
                prefetch_helper(
                    opt_qs_payload.proof_with_data(),
                    self.batch_reader.clone(),
                    None,
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
            Payload::QuorumStoreInlineHybrid(inline_batches, proofs, _) => {
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
        block_signers: Option<BitVec>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>)> {
        let Some(payload) = block.payload() else {
            return Ok((Vec::new(), None));
        };

        let transaction_payload = match payload {
            Payload::InQuorumStore(proof_with_data) => {
                let transactions = process_qs_payload(
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
                let transactions = process_qs_payload(
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
                )
            },
            Payload::QuorumStoreInlineHybrid(
                inline_batches,
                proof_with_data,
                max_txns_to_execute,
            ) => {
                let all_transactions = {
                    let mut all_txns = process_qs_payload(
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
                            .flat_map(|(_batch_info, txns)| txns.clone())
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
                    inline_batches,
                )
            },
            Payload::OptQuorumStore(opt_qs_payload) => {
                let opt_batch_txns = process_optqs_payload(
                    opt_qs_payload.opt_batches(),
                    self.batch_reader.clone(),
                    block,
                    &self.ordered_authors,
                    block_signers.as_ref(),
                )
                .await?;
                let proof_batch_txns = process_optqs_payload(
                    opt_qs_payload.proof_with_data(),
                    self.batch_reader.clone(),
                    block,
                    &self.ordered_authors,
                    None,
                )
                .await?;
                let inline_batch_txns = opt_qs_payload.inline_batches().transactions();
                let all_txns = [proof_batch_txns, opt_batch_txns, inline_batch_txns].concat();
                BlockTransactionPayload::new_opt_quorum_store(
                    all_txns,
                    opt_qs_payload.proof_with_data().deref().clone(),
                    opt_qs_payload.max_txns_to_execute(),
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

        Ok((
            transaction_payload.transactions(),
            transaction_payload.transaction_limit(),
        ))
    }
}

async fn process_optqs_payload<T: TDataInfo>(
    data_ptr: &BatchPointer<T>,
    batch_reader: Arc<dyn BatchReader>,
    block: &Block,
    ordered_authors: &[PeerId],
    additional_peers_to_request: Option<&BitVec>,
) -> ExecutorResult<Vec<SignedTransaction>> {
    let mut signers = Vec::new();
    if let Some(peers) = additional_peers_to_request {
        for i in peers.iter_ones() {
            if let Some(author) = ordered_authors.get(i) {
                signers.push(*author);
            }
        }
    }
    if let Some(author) = block.author() {
        signers.push(author);
    }

    let batches_and_responders = data_ptr
        .batch_summary
        .iter()
        .map(|summary| {
            let mut signers = signers.clone();
            signers.append(&mut summary.signers(ordered_authors));

            (summary.info().clone(), signers)
        })
        .collect();

    QuorumStorePayloadManager::request_and_wait_transactions(
        batches_and_responders,
        block.timestamp_usecs(),
        batch_reader,
    )
    .await
}

/// This is deprecated. Use `process_payload_helper` instead after migrating to
/// OptQuorumStore payload
async fn process_qs_payload(
    proof_with_data: &ProofWithData,
    batch_reader: Arc<dyn BatchReader>,
    block: &Block,
    ordered_authors: &[PeerId],
) -> ExecutorResult<Vec<SignedTransaction>> {
    QuorumStorePayloadManager::request_and_wait_transactions(
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
        batch_reader,
    )
    .await
}
