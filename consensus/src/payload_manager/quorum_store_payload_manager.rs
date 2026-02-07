// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    consensus_observer::{
        network::observer_message::{BlockTransactionPayload, ConsensusObserverMessage},
        publisher::consensus_publisher::ConsensusPublisher,
    },
    payload_manager::TPayloadManager,
    quorum_store::{batch_store::BatchReader, quorum_store_coordinator::CoordinatorCommand},
};
use aptos_bitvec::BitVec;
use aptos_config::config::BlockTransactionFilterConfig;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload},
    payload::{BatchPointer, OptQuorumStorePayload, TDataInfo},
    proof_of_store::{BatchInfo, BatchInfoExt, TBatchInfo},
};
use aptos_crypto::HashValue;
use aptos_executor_types::*;
use aptos_logger::prelude::*;
use aptos_types::{transaction::SignedTransaction, PeerId};
use async_trait::async_trait;
use futures::{channel::mpsc::Sender, future::Shared};
use std::{collections::HashMap, future::Future, ops::Deref, pin::Pin, sync::Arc};

pub trait TQuorumStoreCommitNotifier: Send + Sync {
    fn notify(&self, block_timestamp: u64, batches: Vec<BatchInfoExt>);
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
    fn notify(&self, block_timestamp: u64, batches: Vec<BatchInfoExt>) {
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
    enable_payload_v2: bool,
}

impl QuorumStorePayloadManager {
    pub fn new(
        batch_reader: Arc<dyn BatchReader>,
        commit_notifier: Box<dyn TQuorumStoreCommitNotifier>,
        maybe_consensus_publisher: Option<Arc<ConsensusPublisher>>,
        ordered_authors: Vec<PeerId>,
        address_to_validator_index: HashMap<PeerId, usize>,
        enable_payload_v2: bool,
    ) -> Self {
        Self {
            batch_reader,
            commit_notifier,
            maybe_consensus_publisher,
            ordered_authors,
            address_to_validator_index,
            enable_payload_v2,
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
                    unreachable!("OptQuorumStore should be used");
                },
                Payload::OptQuorumStore(OptQuorumStorePayload::V1(p)) => p.get_all_batch_infos(),
                Payload::OptQuorumStore(OptQuorumStorePayload::V2(p)) => p.get_all_batch_infos(),
                // Deprecated variants - return empty
                Payload::DeprecatedInQuorumStore(_)
                | Payload::DeprecatedInQuorumStoreWithLimit(_)
                | Payload::DeprecatedQuorumStoreInlineHybrid(..)
                | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
            })
            .collect();

        self.commit_notifier.notify(block_timestamp, batches);
    }

    fn prefetch_payload_data(&self, payload: &Payload, author: Author, timestamp: u64) {
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
            Payload::DirectMempool(_) => {
                unreachable!()
            },
            Payload::OptQuorumStore(OptQuorumStorePayload::V1(p)) => {
                prefetch_helper(
                    p.opt_batches(),
                    self.batch_reader.clone(),
                    Some(author),
                    timestamp,
                    &self.ordered_authors,
                );
                prefetch_helper(
                    p.proof_with_data(),
                    self.batch_reader.clone(),
                    None,
                    timestamp,
                    &self.ordered_authors,
                )
            },
            Payload::OptQuorumStore(OptQuorumStorePayload::V2(p)) => {
                prefetch_helper(
                    p.opt_batches(),
                    self.batch_reader.clone(),
                    Some(author),
                    timestamp,
                    &self.ordered_authors,
                );
                prefetch_helper(
                    p.proof_with_data(),
                    self.batch_reader.clone(),
                    None,
                    timestamp,
                    &self.ordered_authors,
                )
            },
            // Deprecated variants - no prefetch needed
            Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
        };
    }

    fn check_denied_inline_transactions(
        &self,
        block: &Block,
        block_txn_filter_config: &BlockTransactionFilterConfig,
    ) -> anyhow::Result<()> {
        // If the filter is disabled, return early
        if !block_txn_filter_config.is_enabled() {
            return Ok(());
        }

        // Get the inline transactions for the block proposal
        let inline_transactions = get_inline_transactions(block);
        if inline_transactions.is_empty() {
            return Ok(());
        }

        // Fetch the block metadata
        let block_id = block.id();
        let block_author = block.author();
        let block_epoch = block.epoch();
        let block_timestamp = block.timestamp_usecs();

        // Identify any denied inline transactions
        let block_transaction_filter = block_txn_filter_config.block_transaction_filter();
        let denied_inline_transactions = block_transaction_filter.get_denied_block_transactions(
            block_id,
            block_author,
            block_epoch,
            block_timestamp,
            inline_transactions,
        );
        if !denied_inline_transactions.is_empty() {
            return Err(anyhow::anyhow!(
                "Inline transactions for QuorumStorePayload denied by block transaction filter: {:?}",
                denied_inline_transactions
            ));
        }

        Ok(()) // No transactions were denied
    }

    fn check_payload_availability(&self, block: &Block) -> Result<(), BitVec> {
        let Some(payload) = block.payload() else {
            return Ok(());
        };

        match payload {
            Payload::DirectMempool(_) => {
                unreachable!("QuorumStore doesn't support DirectMempool payload")
            },
            Payload::OptQuorumStore(OptQuorumStorePayload::V1(p)) => {
                let mut missing_authors = BitVec::with_num_bits(self.ordered_authors.len() as u16);
                for batch in p.opt_batches().deref() {
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
            Payload::OptQuorumStore(OptQuorumStorePayload::V2(p)) => {
                let mut missing_authors = BitVec::with_num_bits(self.ordered_authors.len() as u16);
                for batch in p.opt_batches().deref() {
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
            // Deprecated variants - consider available
            Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
        }
    }

    async fn get_transactions(
        &self,
        block: &Block,
        block_signers: Option<BitVec>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>, Option<u64>)> {
        let Some(payload) = block.payload() else {
            return Ok((Vec::new(), None, None));
        };

        let transaction_payload = match payload {
            Payload::OptQuorumStore(OptQuorumStorePayload::V1(opt_qs_payload)) => {
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
                    opt_qs_payload.block_gas_limit(),
                    [
                        opt_qs_payload.opt_batches().deref().clone(),
                        opt_qs_payload.inline_batches().batch_infos(),
                    ]
                    .concat(),
                )
            },
            Payload::OptQuorumStore(OptQuorumStorePayload::V2(opt_qs_payload)) => {
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
                // Use V2 payload type that preserves BatchInfoExt
                let batch_infos: Vec<_> = opt_qs_payload
                    .opt_batches()
                    .iter()
                    .cloned()
                    .chain(opt_qs_payload.inline_batches().batch_infos())
                    .collect();
                BlockTransactionPayload::new_opt_quorum_store_v2(
                    all_txns,
                    opt_qs_payload.proof_with_data().deref().clone(),
                    opt_qs_payload.max_txns_to_execute(),
                    opt_qs_payload.block_gas_limit(),
                    batch_infos,
                )
            },
            // Deprecated or unsupported variants - return empty
            Payload::DirectMempool(_)
            | Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => {
                unreachable!();
            },
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
            transaction_payload.gas_limit(),
        ))
    }
}

/// Extracts and returns all inline transactions from the payload in the given block
fn get_inline_transactions(block: &Block) -> Vec<SignedTransaction> {
    // If the block has no payload, return an empty vector
    let Some(payload) = block.payload() else {
        return vec![];
    };

    // Fetch the inline transactions from the payload
    match payload {
        Payload::OptQuorumStore(OptQuorumStorePayload::V1(p)) => p.inline_batches().transactions(),
        Payload::OptQuorumStore(OptQuorumStorePayload::V2(p)) => p.inline_batches().transactions(),
        // Deprecated or unsupported variants - return empty
        Payload::DirectMempool(_)
        | Payload::DeprecatedInQuorumStore(_)
        | Payload::DeprecatedInQuorumStoreWithLimit(_)
        | Payload::DeprecatedQuorumStoreInlineHybrid(..)
        | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
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
