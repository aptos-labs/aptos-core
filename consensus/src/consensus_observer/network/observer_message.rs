// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::common::error::Error;
use aptos_consensus_types::{
    common::{BatchPayload, Payload},
    payload::InlineBatches,
    pipelined_block::PipelinedBlock,
    proof_of_store::{BatchInfo, ProofCache, ProofOfStore},
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    block_info::{BlockInfo, Round},
    epoch_change::Verifier,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    transaction::SignedTransaction,
};
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
    vec::IntoIter,
};

/// Types of messages that can be sent between the consensus publisher and observer
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConsensusObserverMessage {
    Request(ConsensusObserverRequest),
    Response(ConsensusObserverResponse),
    DirectSend(ConsensusObserverDirectSend),
}

impl ConsensusObserverMessage {
    /// Creates and returns a new ordered block message using the given blocks and ordered proof
    pub fn new_ordered_block_message(
        blocks: Vec<Arc<PipelinedBlock>>,
        ordered_proof: LedgerInfoWithSignatures,
    ) -> ConsensusObserverDirectSend {
        let ordered_block = OrderedBlock::new(blocks, ordered_proof);
        ConsensusObserverDirectSend::OrderedBlock(ordered_block)
    }

    /// Creates and returns a new commit decision message using the given commit decision
    pub fn new_commit_decision_message(
        commit_proof: LedgerInfoWithSignatures,
    ) -> ConsensusObserverDirectSend {
        let commit_decision = CommitDecision::new(commit_proof);
        ConsensusObserverDirectSend::CommitDecision(commit_decision)
    }

    /// Creates and returns a new block payload message using the given block, transactions and limit
    pub fn new_block_payload_message(
        block: BlockInfo,
        transaction_payload: BlockTransactionPayload,
    ) -> ConsensusObserverDirectSend {
        let block_payload = BlockPayload::new(block, transaction_payload);
        ConsensusObserverDirectSend::BlockPayload(block_payload)
    }
}

impl Display for ConsensusObserverMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusObserverMessage::Request(request) => {
                write!(f, "ConsensusObserverRequest: {}", request)
            },
            ConsensusObserverMessage::Response(response) => {
                write!(f, "ConsensusObserverResponse: {}", response)
            },
            ConsensusObserverMessage::DirectSend(direct_send) => {
                write!(f, "ConsensusObserverDirectSend: {}", direct_send)
            },
        }
    }
}

/// Types of requests that can be sent between the consensus publisher and observer
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConsensusObserverRequest {
    Subscribe,
    Unsubscribe,
}

impl ConsensusObserverRequest {
    /// Returns a summary label for the request
    pub fn get_label(&self) -> &'static str {
        match self {
            ConsensusObserverRequest::Subscribe => "subscribe",
            ConsensusObserverRequest::Unsubscribe => "unsubscribe",
        }
    }
}

impl Display for ConsensusObserverRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_label())
    }
}

/// Types of responses that can be sent between the consensus publisher and observer
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConsensusObserverResponse {
    SubscribeAck,
    UnsubscribeAck,
}

impl ConsensusObserverResponse {
    /// Returns a summary label for the response
    pub fn get_label(&self) -> &'static str {
        match self {
            ConsensusObserverResponse::SubscribeAck => "subscribe_ack",
            ConsensusObserverResponse::UnsubscribeAck => "unsubscribe_ack",
        }
    }
}

impl Display for ConsensusObserverResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_label())
    }
}

/// Types of direct sends that can be sent between the consensus publisher and observer
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConsensusObserverDirectSend {
    OrderedBlock(OrderedBlock),
    CommitDecision(CommitDecision),
    BlockPayload(BlockPayload),
    OrderedBlockWithWindow(OrderedBlockWithWindow),
}

impl ConsensusObserverDirectSend {
    /// Returns a summary label for the direct send
    pub fn get_label(&self) -> &'static str {
        match self {
            ConsensusObserverDirectSend::OrderedBlock(_) => "ordered_block",
            ConsensusObserverDirectSend::CommitDecision(_) => "commit_decision",
            ConsensusObserverDirectSend::BlockPayload(_) => "block_payload",
            ConsensusObserverDirectSend::OrderedBlockWithWindow(_) => "ordered_block_with_window",
        }
    }
}

impl Display for ConsensusObserverDirectSend {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusObserverDirectSend::OrderedBlock(ordered_block) => {
                write!(f, "OrderedBlock: {}", ordered_block.proof_block_info())
            },
            ConsensusObserverDirectSend::CommitDecision(commit_decision) => {
                write!(f, "CommitDecision: {}", commit_decision.proof_block_info())
            },
            ConsensusObserverDirectSend::BlockPayload(block_payload) => {
                write!(
                    f,
                    "BlockPayload: {}. Number of transactions: {}, limit: {:?}, proofs: {:?}",
                    block_payload.block,
                    block_payload.transaction_payload.transactions().len(),
                    block_payload.transaction_payload.transaction_limit(),
                    block_payload.transaction_payload.payload_proofs(),
                )
            },
            ConsensusObserverDirectSend::OrderedBlockWithWindow(ordered_block_with_window) => {
                write!(
                    f,
                    "OrderedBlockWithWindow: {}",
                    ordered_block_with_window.ordered_block.proof_block_info(),
                )
            },
        }
    }
}

/// OrderedBlock message contains the ordered blocks and the proof of the ordering
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OrderedBlock {
    blocks: Vec<Arc<PipelinedBlock>>,
    ordered_proof: LedgerInfoWithSignatures,
}

impl OrderedBlock {
    pub fn new(blocks: Vec<Arc<PipelinedBlock>>, ordered_proof: LedgerInfoWithSignatures) -> Self {
        Self {
            blocks,
            ordered_proof,
        }
    }

    /// Returns a reference to the ordered blocks
    pub fn blocks(&self) -> &Vec<Arc<PipelinedBlock>> {
        &self.blocks
    }

    /// Returns a copy of the first ordered block
    pub fn first_block(&self) -> Arc<PipelinedBlock> {
        self.blocks
            .first()
            .cloned()
            .expect("At least one block is expected!")
    }

    /// Returns a copy of the last ordered block
    pub fn last_block(&self) -> Arc<PipelinedBlock> {
        self.blocks
            .last()
            .cloned()
            .expect("At least one block is expected!")
    }

    /// Returns a reference to the ordered proof
    pub fn ordered_proof(&self) -> &LedgerInfoWithSignatures {
        &self.ordered_proof
    }

    /// Returns a reference to the ordered proof block info
    pub fn proof_block_info(&self) -> &BlockInfo {
        self.ordered_proof.commit_info()
    }

    /// Verifies the ordered blocks and returns an error if the data is invalid.
    /// Note: this does not check the ordered proof.
    pub fn verify_ordered_blocks(&self) -> Result<(), Error> {
        // Verify that we have at least one ordered block
        if self.blocks.is_empty() {
            return Err(Error::InvalidMessageError(
                "Received empty ordered block!".to_string(),
            ));
        }

        // Verify the last block ID matches the ordered proof block ID
        if self.last_block().id() != self.proof_block_info().id() {
            return Err(Error::InvalidMessageError(
                format!(
                    "Last ordered block ID does not match the ordered proof ID! Number of blocks: {:?}, Last ordered block ID: {:?}, Ordered proof ID: {:?}",
                    self.blocks.len(),
                    self.last_block().id(),
                    self.proof_block_info().id()
                )
            ));
        }

        // Verify the blocks are correctly chained together (from the last block to the first)
        let mut expected_parent_id = None;
        for block in self.blocks.iter().rev() {
            if let Some(expected_parent_id) = expected_parent_id {
                if block.id() != expected_parent_id {
                    return Err(Error::InvalidMessageError(
                        format!(
                            "Block parent ID does not match the expected parent ID! Block ID: {:?}, Expected parent ID: {:?}",
                            block.id(),
                            expected_parent_id
                        )
                    ));
                }
            }

            expected_parent_id = Some(block.parent_id());
        }

        Ok(())
    }

    /// Verifies the ordered proof and returns an error if the proof is invalid
    pub fn verify_ordered_proof(&self, epoch_state: &EpochState) -> Result<(), Error> {
        epoch_state.verify(&self.ordered_proof).map_err(|error| {
            Error::InvalidMessageError(format!(
                "Failed to verify ordered proof ledger info: {:?}, Error: {:?}",
                self.proof_block_info(),
                error
            ))
        })
    }
}

/// OrderedBlockWithWindow message contains the ordered blocks, and
/// the window information (e.g., dependencies for execution pool).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OrderedBlockWithWindow {
    ordered_block: OrderedBlock,
    execution_pool_window: ExecutionPoolWindow,
}

impl OrderedBlockWithWindow {
    pub fn new(ordered_block: OrderedBlock, execution_pool_window: ExecutionPoolWindow) -> Self {
        Self {
            ordered_block,
            execution_pool_window,
        }
    }

    /// Returns a reference to the execution pool window
    pub fn execution_pool_window(&self) -> &ExecutionPoolWindow {
        &self.execution_pool_window
    }

    /// Consumes the ordered block with window and returns the inner parts
    pub fn into_parts(self) -> (OrderedBlock, ExecutionPoolWindow) {
        (self.ordered_block, self.execution_pool_window)
    }

    /// Returns a reference to the ordered block
    pub fn ordered_block(&self) -> &OrderedBlock {
        &self.ordered_block
    }
}

/// The execution pool window information for an ordered block
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutionPoolWindow {
    // TODO: identify exactly what information is required here
    block_ids: Vec<HashValue>, // The list of parent block hashes in chronological order
}

impl ExecutionPoolWindow {
    pub fn new(block_ids: Vec<HashValue>) -> Self {
        Self { block_ids }
    }

    /// Returns a reference to the block IDs in the execution pool window
    pub fn block_ids(&self) -> &Vec<HashValue> {
        &self.block_ids
    }

    /// Verifies the execution pool window contents and returns an error if the data is invalid
    pub fn verify_window_contents(&self, _expected_window_size: u64) -> Result<(), Error> {
        Ok(()) // TODO: Implement this method!
    }
}

/// CommitDecision message contains the commit decision proof
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CommitDecision {
    commit_proof: LedgerInfoWithSignatures,
}

impl CommitDecision {
    pub fn new(commit_proof: LedgerInfoWithSignatures) -> Self {
        Self { commit_proof }
    }

    /// Returns a reference to the commit proof
    pub fn commit_proof(&self) -> &LedgerInfoWithSignatures {
        &self.commit_proof
    }

    /// Returns the epoch of the commit proof
    pub fn epoch(&self) -> u64 {
        self.commit_proof.ledger_info().epoch()
    }

    /// Returns a reference to the commit proof block info
    pub fn proof_block_info(&self) -> &BlockInfo {
        self.commit_proof.commit_info()
    }

    /// Returns the round of the commit proof
    pub fn round(&self) -> Round {
        self.commit_proof.ledger_info().round()
    }

    /// Verifies the commit proof and returns an error if the proof is invalid
    pub fn verify_commit_proof(&self, epoch_state: &EpochState) -> Result<(), Error> {
        epoch_state.verify(&self.commit_proof).map_err(|error| {
            Error::InvalidMessageError(format!(
                "Failed to verify commit proof ledger info: {:?}, Error: {:?}",
                self.proof_block_info(),
                error
            ))
        })
    }
}

/// The transaction payload and proof of each block
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PayloadWithProof {
    transactions: Vec<SignedTransaction>,
    proofs: Vec<ProofOfStore>,
}

impl PayloadWithProof {
    pub fn new(transactions: Vec<SignedTransaction>, proofs: Vec<ProofOfStore>) -> Self {
        Self {
            transactions,
            proofs,
        }
    }

    #[cfg(test)]
    /// Returns an empty payload with proof (for testing)
    pub fn empty() -> Self {
        Self {
            transactions: vec![],
            proofs: vec![],
        }
    }
}

/// The transaction payload and proof of each block with a transaction limit
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PayloadWithProofAndLimit {
    payload_with_proof: PayloadWithProof,
    transaction_limit: Option<u64>,
}

impl PayloadWithProofAndLimit {
    pub fn new(payload_with_proof: PayloadWithProof, limit: Option<u64>) -> Self {
        Self {
            payload_with_proof,
            transaction_limit: limit,
        }
    }

    #[cfg(test)]
    /// Returns an empty payload with proof and limit (for testing)
    pub fn empty() -> Self {
        Self {
            payload_with_proof: PayloadWithProof::empty(),
            transaction_limit: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TransactionsWithProof {
    TransactionsWithProofAndLimits(TransactionsWithProofAndLimits),
}

impl TransactionsWithProof {
    pub fn transactions(&self) -> Vec<SignedTransaction> {
        match self {
            TransactionsWithProof::TransactionsWithProofAndLimits(payload) => {
                payload.payload_with_proof.transactions.clone()
            },
        }
    }

    pub fn proofs(&self) -> Vec<ProofOfStore> {
        match self {
            TransactionsWithProof::TransactionsWithProofAndLimits(payload) => {
                payload.payload_with_proof.proofs.clone()
            },
        }
    }

    pub fn transaction_limit(&self) -> Option<u64> {
        match self {
            TransactionsWithProof::TransactionsWithProofAndLimits(payload) => {
                payload.transaction_limit
            },
        }
    }

    pub fn gas_limit(&self) -> Option<u64> {
        match self {
            TransactionsWithProof::TransactionsWithProofAndLimits(payload) => payload.gas_limit,
        }
    }
}

/// The transaction payload and proof of each block with a transaction and block gas limit
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransactionsWithProofAndLimits {
    payload_with_proof: PayloadWithProof,
    transaction_limit: Option<u64>,
    gas_limit: Option<u64>,
}

impl TransactionsWithProofAndLimits {
    pub fn new(
        payload_with_proof: PayloadWithProof,
        transaction_limit: Option<u64>,
        gas_limit: Option<u64>,
    ) -> Self {
        Self {
            payload_with_proof,
            transaction_limit,
            gas_limit,
        }
    }

    #[cfg(test)]
    /// Returns an empty payload with proof and limit (for testing)
    pub fn empty() -> Self {
        Self {
            payload_with_proof: PayloadWithProof::empty(),
            transaction_limit: None,
            gas_limit: None,
        }
    }
}

/// The transaction payload of each block
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockTransactionPayload {
    // TODO: deprecate InQuorumStore* variants
    DeprecatedInQuorumStore(PayloadWithProof),
    DeprecatedInQuorumStoreWithLimit(PayloadWithProofAndLimit),
    QuorumStoreInlineHybrid(PayloadWithProofAndLimit, Vec<BatchInfo>),
    OptQuorumStore(
        TransactionsWithProof,
        /* OptQS and Inline Batches */ Vec<BatchInfo>,
    ),
    QuorumStoreInlineHybridV2(TransactionsWithProof, Vec<BatchInfo>),
}

impl BlockTransactionPayload {
    /// Creates a returns a new InQuorumStore transaction payload
    pub fn new_in_quorum_store(
        transactions: Vec<SignedTransaction>,
        proofs: Vec<ProofOfStore>,
    ) -> Self {
        let payload_with_proof = PayloadWithProof::new(transactions, proofs);
        Self::DeprecatedInQuorumStore(payload_with_proof)
    }

    /// Creates a returns a new InQuorumStoreWithLimit transaction payload
    pub fn new_in_quorum_store_with_limit(
        transactions: Vec<SignedTransaction>,
        proofs: Vec<ProofOfStore>,
        limit: Option<u64>,
    ) -> Self {
        let payload_with_proof = PayloadWithProof::new(transactions, proofs);
        let proof_with_limit = PayloadWithProofAndLimit::new(payload_with_proof, limit);
        Self::DeprecatedInQuorumStoreWithLimit(proof_with_limit)
    }

    /// Creates a returns a new QuorumStoreInlineHybrid transaction payload
    pub fn new_quorum_store_inline_hybrid(
        transactions: Vec<SignedTransaction>,
        proofs: Vec<ProofOfStore>,
        transaction_limit: Option<u64>,
        gas_limit: Option<u64>,
        inline_batches: Vec<BatchInfo>,
        enable_payload_v2: bool,
    ) -> Self {
        let payload_with_proof = PayloadWithProof::new(transactions, proofs);
        if enable_payload_v2 {
            let proof_with_limits = TransactionsWithProof::TransactionsWithProofAndLimits(
                TransactionsWithProofAndLimits::new(
                    payload_with_proof,
                    transaction_limit,
                    gas_limit,
                ),
            );
            Self::QuorumStoreInlineHybridV2(proof_with_limits, inline_batches)
        } else {
            let proof_with_limit =
                PayloadWithProofAndLimit::new(payload_with_proof, transaction_limit);
            Self::QuorumStoreInlineHybrid(proof_with_limit, inline_batches)
        }
    }

    pub fn new_opt_quorum_store(
        transactions: Vec<SignedTransaction>,
        proofs: Vec<ProofOfStore>,
        limit: Option<u64>,
        batch_infos: Vec<BatchInfo>,
    ) -> Self {
        let payload_with_proof = PayloadWithProof::new(transactions, proofs);
        let proof_with_limits = TransactionsWithProof::TransactionsWithProofAndLimits(
            TransactionsWithProofAndLimits::new(payload_with_proof, limit, None),
        );
        Self::OptQuorumStore(proof_with_limits, batch_infos)
    }

    #[cfg(test)]
    /// Returns an empty transaction payload (for testing)
    pub fn empty() -> Self {
        Self::QuorumStoreInlineHybrid(PayloadWithProofAndLimit::empty(), vec![])
    }

    /// Returns the list of inline batches and optimistic batches in the transaction payload
    pub fn optqs_and_inline_batches(&self) -> &[BatchInfo] {
        match self {
            BlockTransactionPayload::DeprecatedInQuorumStore(_)
            | BlockTransactionPayload::DeprecatedInQuorumStoreWithLimit(_) => &[],
            BlockTransactionPayload::QuorumStoreInlineHybrid(_, inline_batches)
            | BlockTransactionPayload::QuorumStoreInlineHybridV2(_, inline_batches)
            | BlockTransactionPayload::OptQuorumStore(_, inline_batches) => inline_batches,
        }
    }

    /// Returns the transaction limit of the payload
    pub fn transaction_limit(&self) -> Option<u64> {
        match self {
            BlockTransactionPayload::DeprecatedInQuorumStore(_) => None,
            BlockTransactionPayload::DeprecatedInQuorumStoreWithLimit(payload) => {
                payload.transaction_limit
            },
            BlockTransactionPayload::QuorumStoreInlineHybrid(payload, _) => {
                payload.transaction_limit
            },
            BlockTransactionPayload::QuorumStoreInlineHybridV2(payload, _)
            | BlockTransactionPayload::OptQuorumStore(payload, _) => payload.transaction_limit(),
        }
    }

    /// Returns the block gas limit of the payload
    pub fn gas_limit(&self) -> Option<u64> {
        match self {
            BlockTransactionPayload::DeprecatedInQuorumStore(_)
            | BlockTransactionPayload::DeprecatedInQuorumStoreWithLimit(_)
            | BlockTransactionPayload::QuorumStoreInlineHybrid(_, _) => None,
            BlockTransactionPayload::QuorumStoreInlineHybridV2(payload, _)
            | BlockTransactionPayload::OptQuorumStore(payload, _) => payload.gas_limit(),
        }
    }

    /// Returns the proofs of the transaction payload
    pub fn payload_proofs(&self) -> Vec<ProofOfStore> {
        match self {
            BlockTransactionPayload::DeprecatedInQuorumStore(payload) => payload.proofs.clone(),
            BlockTransactionPayload::DeprecatedInQuorumStoreWithLimit(payload) => {
                payload.payload_with_proof.proofs.clone()
            },
            BlockTransactionPayload::QuorumStoreInlineHybrid(payload, _) => {
                payload.payload_with_proof.proofs.clone()
            },
            BlockTransactionPayload::QuorumStoreInlineHybridV2(payload, _)
            | BlockTransactionPayload::OptQuorumStore(payload, _) => payload.proofs(),
        }
    }

    /// Returns the transactions in the payload
    pub fn transactions(&self) -> Vec<SignedTransaction> {
        match self {
            BlockTransactionPayload::DeprecatedInQuorumStore(payload) => {
                payload.transactions.clone()
            },
            BlockTransactionPayload::DeprecatedInQuorumStoreWithLimit(payload) => {
                payload.payload_with_proof.transactions.clone()
            },
            BlockTransactionPayload::QuorumStoreInlineHybrid(payload, _) => {
                payload.payload_with_proof.transactions.clone()
            },
            BlockTransactionPayload::QuorumStoreInlineHybridV2(payload, _)
            | BlockTransactionPayload::OptQuorumStore(payload, _) => payload.transactions(),
        }
    }

    /// Verifies the transaction payload against the given ordered block payload
    pub fn verify_against_ordered_payload(
        &self,
        ordered_block_payload: &Payload,
    ) -> Result<(), Error> {
        match ordered_block_payload {
            Payload::DirectMempool(_) => {
                return Err(Error::InvalidMessageError(
                    "Direct mempool payloads are not supported for consensus observer!".into(),
                ));
            },
            Payload::InQuorumStore(proof_with_data) => {
                // Verify the batches in the requested block
                self.verify_batches(&proof_with_data.proofs)?;
            },
            Payload::InQuorumStoreWithLimit(proof_with_data) => {
                // Verify the batches in the requested block
                self.verify_batches(&proof_with_data.proof_with_data.proofs)?;

                // Verify the transaction limit
                self.verify_transaction_limit(proof_with_data.max_txns_to_execute)?;
            },
            Payload::QuorumStoreInlineHybrid(
                inline_batches,
                proof_with_data,
                max_txns_to_execute,
            ) => {
                // Verify the batches in the requested block
                self.verify_batches(&proof_with_data.proofs)?;

                // Verify the inline batches
                self.verify_inline_batches(inline_batches)?;

                // Verify the transaction limit
                self.verify_transaction_limit(*max_txns_to_execute)?;
            },
            Payload::QuorumStoreInlineHybridV2(
                inline_batches,
                proof_with_data,
                execution_limits,
            ) => {
                // Verify the batches in the requested block
                self.verify_batches(&proof_with_data.proofs)?;

                // Verify the inline batches
                self.verify_inline_batches(inline_batches)?;

                // Verify the transaction limit
                self.verify_transaction_limit(execution_limits.max_txns_to_execute())?;

                // TODO: verify the block gas limit?
            },
            Payload::OptQuorumStore(_) | Payload::MoonBlock(_) | Payload::EarthBlock(_) => {
                let opt_qs_payload = ordered_block_payload
                    .as_opt_qs_payload()
                    .expect("Should have OptQuorumStore payload");
                // Verify the batches in the requested block
                self.verify_batches(opt_qs_payload.proof_with_data())?;

                // Verify optQS and inline batches
                self.verify_optqs_and_inline_batches(
                    opt_qs_payload.opt_batches(),
                    opt_qs_payload.inline_batches(),
                )?;

                // Verify the transaction limit
                self.verify_transaction_limit(opt_qs_payload.max_txns_to_execute())?;
            },
        }

        Ok(())
    }

    /// Verifies the payload batches against the expected batches
    fn verify_batches(&self, expected_proofs: &[ProofOfStore]) -> Result<(), Error> {
        // Get the batches in the block transaction payload
        let payload_proofs = self.payload_proofs();
        let payload_batches: Vec<&BatchInfo> =
            payload_proofs.iter().map(|proof| proof.info()).collect();

        // Compare the expected batches against the payload batches
        let expected_batches: Vec<&BatchInfo> =
            expected_proofs.iter().map(|proof| proof.info()).collect();
        if expected_batches != payload_batches {
            return Err(Error::InvalidMessageError(format!(
                "Transaction payload failed batch verification! Expected batches {:?}, but found {:?}!",
                expected_batches, payload_batches
            )));
        }

        Ok(())
    }

    /// Verifies the inline batches against the expected inline batches
    fn verify_inline_batches(
        &self,
        expected_inline_batches: &[(BatchInfo, Vec<SignedTransaction>)],
    ) -> Result<(), Error> {
        // Get the expected inline batches
        let expected_inline_batches: Vec<&BatchInfo> = expected_inline_batches
            .iter()
            .map(|(batch_info, _)| batch_info)
            .collect();

        // Get the inline batches in the payload
        let inline_batches: Vec<&BatchInfo> = match self {
            BlockTransactionPayload::QuorumStoreInlineHybrid(_, inline_batches)
            | BlockTransactionPayload::QuorumStoreInlineHybridV2(_, inline_batches) => {
                inline_batches.iter().collect()
            },
            _ => {
                return Err(Error::InvalidMessageError(
                    "Transaction payload does not contain inline batches!".to_string(),
                ))
            },
        };

        // Compare the expected inline batches against the payload inline batches
        if expected_inline_batches != inline_batches {
            return Err(Error::InvalidMessageError(format!(
                "Transaction payload failed inline batch verification! Expected inline batches {:?} but found {:?}",
                expected_inline_batches, inline_batches
            )));
        }

        Ok(())
    }

    fn verify_optqs_and_inline_batches(
        &self,
        expected_opt_batches: &Vec<BatchInfo>,
        expected_inline_batches: &InlineBatches,
    ) -> Result<(), Error> {
        let optqs_and_inline_batches: &Vec<BatchInfo> = match self {
            BlockTransactionPayload::OptQuorumStore(_, optqs_and_inline_batches) => {
                optqs_and_inline_batches
            },
            _ => {
                return Err(Error::InvalidMessageError(
                    "Transaction payload is not an OptQS Payload".to_string(),
                ))
            },
        };

        let expected_opt_and_inline_batches = expected_opt_batches.iter().chain(
            expected_inline_batches
                .iter()
                .map(|inline_batch| inline_batch.info()),
        );

        if !expected_opt_and_inline_batches.eq(optqs_and_inline_batches.iter()) {
            return Err(Error::InvalidMessageError(format!(
                "Transaction payload failed batch verification! Expected optimistic batches {:?}, inline batches {:?} but found {:?}",
                expected_opt_batches, expected_inline_batches, optqs_and_inline_batches
            )));
        }
        Ok(())
    }

    /// Verifies the payload limit against the expected limit
    fn verify_transaction_limit(
        &self,
        expected_transaction_limit: Option<u64>,
    ) -> Result<(), Error> {
        // Get the payload limit
        let limit = match self {
            BlockTransactionPayload::DeprecatedInQuorumStore(_) => {
                return Err(Error::InvalidMessageError(
                    "Transaction payload does not contain a limit!".to_string(),
                ))
            },
            BlockTransactionPayload::DeprecatedInQuorumStoreWithLimit(payload) => {
                payload.transaction_limit
            },
            BlockTransactionPayload::QuorumStoreInlineHybrid(payload, _) => {
                payload.transaction_limit
            },
            BlockTransactionPayload::QuorumStoreInlineHybridV2(payload, _)
            | BlockTransactionPayload::OptQuorumStore(payload, _) => payload.transaction_limit(),
        };

        // Compare the expected limit against the payload limit
        if expected_transaction_limit != limit {
            return Err(Error::InvalidMessageError(format!(
                "Transaction payload failed limit verification! Expected limit: {:?}, Found limit: {:?}",
                expected_transaction_limit, limit
            )));
        }

        Ok(())
    }
}

/// Payload message contains the block and transaction payload
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockPayload {
    block: BlockInfo,
    transaction_payload: BlockTransactionPayload,
}

impl BlockPayload {
    pub fn new(block: BlockInfo, transaction_payload: BlockTransactionPayload) -> Self {
        Self {
            block,
            transaction_payload,
        }
    }

    /// Returns a reference to the block info
    pub fn block(&self) -> &BlockInfo {
        &self.block
    }

    /// Returns the epoch of the block info
    pub fn epoch(&self) -> u64 {
        self.block.epoch()
    }

    /// Returns the round of the block info
    pub fn round(&self) -> Round {
        self.block.round()
    }

    /// Returns a reference to the block transaction payload
    pub fn transaction_payload(&self) -> &BlockTransactionPayload {
        &self.transaction_payload
    }

    /// Verifies the block payload digests and returns an error if the data is invalid
    pub fn verify_payload_digests(&self) -> Result<(), Error> {
        // Get the block info, transactions, payload proofs and inline batches
        let block_info = self.block.clone();
        let transactions = self.transaction_payload.transactions();
        let payload_proofs = self.transaction_payload.payload_proofs();
        let opt_and_inline_batches = self.transaction_payload.optqs_and_inline_batches();

        // Get the number of transactions, payload proofs and inline batches
        let num_transactions = transactions.len();
        let num_payload_proofs = payload_proofs.len();
        let num_opt_and_inline_batches = opt_and_inline_batches.len();

        // Gather the transactions for each payload batch
        let mut batches_and_transactions = vec![];
        let mut transactions_iter = transactions.into_iter();
        for proof_of_store in &payload_proofs {
            match reconstruct_batch(
                &block_info,
                &mut transactions_iter,
                proof_of_store.info(),
                true,
            ) {
                Ok(Some(batch_transactions)) => {
                    batches_and_transactions
                        .push((proof_of_store.info().clone(), batch_transactions));
                },
                Ok(None) => { /* Nothing needs to be done (the batch was expired) */ },
                Err(error) => {
                    return Err(Error::InvalidMessageError(format!(
                        "Failed to reconstruct payload proof batch! Num transactions: {:?}, \
                        num batches: {:?}, num inline batches: {:?}, failed batch: {:?}, Error: {:?}",
                        num_transactions, num_payload_proofs, num_opt_and_inline_batches, proof_of_store.info(), error
                    )));
                },
            }
        }

        // Gather the transactions for each inline batch
        for batch_info in opt_and_inline_batches.iter() {
            match reconstruct_batch(&block_info, &mut transactions_iter, batch_info, false) {
                Ok(Some(batch_transactions)) => {
                    batches_and_transactions.push((batch_info.clone(), batch_transactions));
                },
                Ok(None) => {
                    return Err(Error::UnexpectedError(format!(
                        "Failed to reconstruct inline/opt batch! Batch was unexpectedly skipped: {:?}",
                        batch_info
                    )));
                },
                Err(error) => {
                    return Err(Error::InvalidMessageError(format!(
                        "Failed to reconstruct inline/opt batch! Num transactions: {:?}, \
                        num batches: {:?}, num opt/inline batches: {:?}, failed batch: {:?}, Error: {:?}",
                        num_transactions, num_payload_proofs, num_opt_and_inline_batches, batch_info, error
                    )));
                },
            }
        }

        // Verify all the reconstructed batches (in parallel)
        batches_and_transactions
            .into_par_iter()
            .with_min_len(2)
            .try_for_each(|(batch_info, transactions)| verify_batch(&batch_info, transactions))
            .map_err(|error| {
                Error::InvalidMessageError(format!(
                    "Failed to verify the payload batches and transactions! Error: {:?}",
                    error
                ))
            })?;

        // Verify that there are no transactions remaining (all transactions should be consumed)
        let remaining_transactions = transactions_iter.as_slice();
        if !remaining_transactions.is_empty() {
            return Err(Error::InvalidMessageError(format!(
                "Failed to verify payload transactions! Num transactions: {:?}, \
                transactions remaining: {:?}. Expected: 0",
                num_transactions,
                remaining_transactions.len()
            )));
        }

        Ok(()) // All digests match
    }

    /// Verifies that the block payload proofs are correctly signed according
    /// to the current epoch state. Returns an error if the data is invalid.
    pub fn verify_payload_signatures(&self, epoch_state: &EpochState) -> Result<(), Error> {
        // Create a dummy proof cache to verify the proofs
        let proof_cache = ProofCache::new(1);

        // Verify each of the proof signatures (in parallel)
        let payload_proofs = self.transaction_payload.payload_proofs();
        let validator_verifier = &epoch_state.verifier;
        payload_proofs
            .par_iter()
            .with_min_len(2)
            .try_for_each(|proof| proof.verify(validator_verifier, &proof_cache))
            .map_err(|error| {
                Error::InvalidMessageError(format!(
                    "Failed to verify the payload proof signatures! Error: {:?}",
                    error
                ))
            })?;

        Ok(()) // All proofs are correctly signed
    }
}

/// Reconstructs the batch using the given transactions and the
/// expected batch info. If `skip_expired_batches` is true
/// then reconstruction will be skipped if the batch is expired.
fn reconstruct_batch(
    block_info: &BlockInfo,
    transactions_iter: &mut IntoIter<SignedTransaction>,
    expected_batch_info: &BatchInfo,
    skip_expired_batches: bool,
) -> Result<Option<Vec<SignedTransaction>>, Error> {
    // If the batch is expired we should skip reconstruction (as the
    // transactions for the expired batch won't be sent in the payload).
    // Note: this should only be required for QS batches (not inline batches).
    if skip_expired_batches && block_info.timestamp_usecs() > expected_batch_info.expiration() {
        return Ok(None);
    }

    // Gather the transactions for the batch
    let mut batch_transactions = vec![];
    for i in 0..expected_batch_info.num_txns() {
        let batch_transaction = match transactions_iter.next() {
            Some(transaction) => transaction,
            None => {
                return Err(Error::InvalidMessageError(format!(
                    "Failed to extract transaction during batch reconstruction! Batch: {:?}, transaction index: {:?}",
                    expected_batch_info, i
                )));
            },
        };
        batch_transactions.push(batch_transaction);
    }

    Ok(Some(batch_transactions))
}

/// Verifies the batch digest using the given transactions and the expected batch info
fn verify_batch(
    expected_batch_info: &BatchInfo,
    batch_transactions: Vec<SignedTransaction>,
) -> Result<(), Error> {
    // Calculate the batch digest
    let batch_payload = BatchPayload::new(expected_batch_info.author(), batch_transactions);
    let batch_digest = batch_payload.hash();

    // Verify the reconstructed digest against the expected digest
    let expected_digest = expected_batch_info.digest();
    if batch_digest != *expected_digest {
        return Err(Error::InvalidMessageError(format!(
            "The reconstructed batch digest does not match the expected digest! \
             Batch: {:?}, Expected digest: {:?}, Reconstructed digest: {:?}",
            expected_batch_info, expected_digest, batch_digest
        )));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_bitvec::BitVec;
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        common::{Author, ProofWithData, ProofWithDataWithTxnLimit},
        payload::{
            BatchPointer, InlineBatch, OptBatches, OptQuorumStorePayload, PayloadExecutionLimit,
            ProofBatches,
        },
        pipelined_block::OrderedBlockWindow,
        quorum_cert::QuorumCert,
    };
    use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        chain_id::ChainId,
        ledger_info::LedgerInfo,
        quorum_store::BatchId,
        transaction::{RawTransaction, Script, TransactionPayload},
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
        PeerId,
    };
    use claims::{assert_matches, assert_ok};
    use move_core_types::account_address::AccountAddress;
    use std::ops::Deref;

    #[test]
    fn test_verify_against_ordered_payload_mempool() {
        // Create an empty transaction payload
        let transaction_payload = BlockTransactionPayload::new_in_quorum_store(vec![], vec![]);

        // Create a direct mempool payload
        let ordered_payload = Payload::DirectMempool(vec![]);

        // Verify the transaction payload and ensure it fails (mempool payloads are not supported)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));
    }

    #[test]
    fn test_verify_against_ordered_payload_in_qs() {
        // Create an empty transaction payload with no proofs
        let proofs = vec![];
        let transaction_payload =
            BlockTransactionPayload::new_in_quorum_store(vec![], proofs.clone());

        // Create a quorum store payload with a single proof
        let batch_info = create_batch_info();
        let proof_with_data = ProofWithData::new(vec![ProofOfStore::new(
            batch_info,
            AggregateSignature::empty(),
        )]);
        let ordered_payload = Payload::InQuorumStore(proof_with_data);

        // Verify the transaction payload and ensure it fails (the batch infos don't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a quorum store payload with no proofs
        let proof_with_data = ProofWithData::new(proofs);
        let ordered_payload = Payload::InQuorumStore(proof_with_data);

        // Verify the transaction payload and ensure it passes
        transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap();
    }

    #[test]
    fn test_verify_against_ordered_payload_in_qs_limit() {
        // Create an empty transaction payload with no proofs
        let proofs = vec![];
        let transaction_limit = Some(10);
        let transaction_payload = BlockTransactionPayload::new_in_quorum_store_with_limit(
            vec![],
            proofs.clone(),
            transaction_limit,
        );

        // Create a quorum store payload with a single proof
        let batch_info = create_batch_info();
        let proof_with_data = ProofWithDataWithTxnLimit::new(
            ProofWithData::new(vec![ProofOfStore::new(
                batch_info,
                AggregateSignature::empty(),
            )]),
            transaction_limit,
        );
        let ordered_payload = Payload::InQuorumStoreWithLimit(proof_with_data);

        // Verify the transaction payload and ensure it fails (the batch infos don't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a quorum store payload with no proofs and no transaction limit
        let proof_with_data =
            ProofWithDataWithTxnLimit::new(ProofWithData::new(proofs.clone()), None);
        let ordered_payload = Payload::InQuorumStoreWithLimit(proof_with_data);

        // Verify the transaction payload and ensure it fails (the transaction limit doesn't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a quorum store payload with no proofs and the correct limit
        let proof_with_data =
            ProofWithDataWithTxnLimit::new(ProofWithData::new(proofs), transaction_limit);
        let ordered_payload = Payload::InQuorumStoreWithLimit(proof_with_data);

        // Verify the transaction payload and ensure it passes
        transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap();
    }

    #[test]
    fn test_verify_against_ordered_payload_in_qs_hybrid() {
        // Create an empty transaction payload with no proofs and no inline batches
        let proofs = vec![];
        let transaction_limit = Some(100);
        let gas_limit = Some(10_000);
        let inline_batches = vec![];
        let transaction_payload = BlockTransactionPayload::new_quorum_store_inline_hybrid(
            vec![],
            proofs.clone(),
            transaction_limit,
            gas_limit,
            inline_batches.clone(),
            true,
        );

        // Create a quorum store payload with a single proof
        let inline_batches = vec![];
        let batch_info = create_batch_info();
        let proof_with_data = ProofWithData::new(vec![ProofOfStore::new(
            batch_info,
            AggregateSignature::empty(),
        )]);
        let ordered_payload = Payload::QuorumStoreInlineHybrid(
            inline_batches.clone(),
            proof_with_data,
            transaction_limit,
        );

        // Verify the transaction payload and ensure it fails (the batch infos don't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a quorum store payload with no transaction limit
        let proof_with_data = ProofWithData::new(vec![]);
        let ordered_payload =
            Payload::QuorumStoreInlineHybrid(inline_batches.clone(), proof_with_data, None);

        // Verify the transaction payload and ensure it fails (the transaction limit doesn't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a quorum store payload with a single inline batch
        let proof_with_data = ProofWithData::new(vec![]);
        let ordered_payload = Payload::QuorumStoreInlineHybrid(
            vec![(create_batch_info(), vec![])],
            proof_with_data,
            transaction_limit,
        );

        // Verify the transaction payload and ensure it fails (the inline batches don't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create an empty quorum store payload
        let proof_with_data = ProofWithData::new(vec![]);
        let ordered_payload =
            Payload::QuorumStoreInlineHybrid(vec![], proof_with_data, transaction_limit);

        // Verify the transaction payload and ensure it passes
        transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap();
    }

    #[test]
    fn test_verify_against_ordered_payload_optqs() {
        // Create an empty transaction payload with no proofs and no inline batches
        let proofs = vec![];
        let transaction_limit = Some(100);
        let opt_and_inline_batches = vec![];
        let transaction_payload = BlockTransactionPayload::new_opt_quorum_store(
            vec![],
            proofs.clone(),
            transaction_limit,
            opt_and_inline_batches.clone(),
        );

        // Create a quorum store payload with a single proof
        let inline_batches = InlineBatches::from(Vec::<InlineBatch>::new());
        let opt_batches: BatchPointer<BatchInfo> = Vec::new().into();
        let batch_info = create_batch_info();
        let proof_with_data: ProofBatches =
            vec![ProofOfStore::new(batch_info, AggregateSignature::empty())].into();
        let ordered_payload = Payload::OptQuorumStore(OptQuorumStorePayload::new(
            inline_batches.clone(),
            opt_batches.clone(),
            proof_with_data,
            PayloadExecutionLimit::None,
        ));

        // Verify the transaction payload and ensure it fails (the batch infos don't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a quorum store payload with no transaction limit
        let proof_with_data: ProofBatches = Vec::new().into();
        let ordered_payload = Payload::OptQuorumStore(OptQuorumStorePayload::new(
            inline_batches,
            opt_batches,
            proof_with_data,
            PayloadExecutionLimit::None,
        ));

        // Verify the transaction payload and ensure it fails (the transaction limit doesn't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a quorum store payload with a single inline batch
        let proof_with_data: ProofBatches = Vec::new().into();
        let ordered_payload = Payload::OptQuorumStore(OptQuorumStorePayload::new(
            vec![(create_batch_info(), vec![])].into(),
            Vec::new().into(),
            proof_with_data,
            PayloadExecutionLimit::None,
        ));

        // Verify the transaction payload and ensure it fails (the inline batches don't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a quorum store payload with a single opt batch
        let proof_with_data: ProofBatches = Vec::new().into();
        let ordered_payload = Payload::OptQuorumStore(OptQuorumStorePayload::new(
            Vec::<InlineBatch>::new().into(),
            vec![create_batch_info()].into(),
            proof_with_data,
            PayloadExecutionLimit::None,
        ));

        // Verify the transaction payload and ensure it fails (the opt batches don't match)
        let error = transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create an empty opt quorum store payload
        let proof_with_data = Vec::new().into();
        let ordered_payload = Payload::OptQuorumStore(OptQuorumStorePayload::new(
            Vec::<InlineBatch>::new().into(),
            Vec::new().into(),
            proof_with_data,
            PayloadExecutionLimit::MaxTransactionsToExecute(100),
        ));

        // Verify the transaction payload and ensure it passes
        transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap();

        // Create an opt quorum store payload with a inline batch, opt batch, and proof batch
        let proofs = vec![ProofOfStore::new(
            create_batch_info(),
            AggregateSignature::empty(),
        )];
        let inline_batches: InlineBatches = vec![(create_batch_info(), vec![])].into();
        let opt_batches: OptBatches = vec![create_batch_info()].into();
        let opt_and_inline_batches =
            [opt_batches.deref().clone(), inline_batches.batch_infos()].concat();

        let ordered_payload = Payload::OptQuorumStore(OptQuorumStorePayload::new(
            inline_batches,
            opt_batches,
            proofs.clone().into(),
            PayloadExecutionLimit::MaxTransactionsToExecute(100),
        ));

        let transaction_payload = BlockTransactionPayload::new_opt_quorum_store(
            vec![],
            proofs,
            Some(100),
            opt_and_inline_batches,
        );

        // Verify the transaction payload and ensure it passes
        transaction_payload
            .verify_against_ordered_payload(&ordered_payload)
            .unwrap();
    }

    #[test]
    fn test_verify_commit_proof() {
        // Create a ledger info with an empty signature set
        let current_epoch = 0;
        let ledger_info = create_empty_ledger_info(current_epoch);

        // Create an epoch state for the current epoch (with an empty verifier)
        let epoch_state = EpochState::new(current_epoch, ValidatorVerifier::new(vec![]));

        // Create a commit decision message with the ledger info
        let commit_decision = CommitDecision::new(ledger_info);

        // Verify the commit proof and ensure it passes
        commit_decision.verify_commit_proof(&epoch_state).unwrap();

        // Create an epoch state for the current epoch (with a non-empty verifier)
        let validator_signer = ValidatorSigner::random(None);
        let validator_consensus_info = ValidatorConsensusInfo::new(
            validator_signer.author(),
            validator_signer.public_key(),
            100,
        );
        let validator_verifier = ValidatorVerifier::new(vec![validator_consensus_info]);
        let epoch_state = EpochState::new(current_epoch, validator_verifier);

        // Verify the commit proof and ensure it fails (the signature set is insufficient)
        let error = commit_decision
            .verify_commit_proof(&epoch_state)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));
    }

    #[test]
    fn test_verify_ordered_blocks() {
        // Create an ordered block with no internal blocks
        let current_epoch = 0;
        let ordered_block = OrderedBlock::new(vec![], create_empty_ledger_info(current_epoch));

        // Verify the ordered blocks and ensure it fails (there are no internal blocks)
        let error = ordered_block.verify_ordered_blocks().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a pipelined block with a random block ID
        let block_id = HashValue::random();
        let block_info = create_block_info(current_epoch, block_id);
        let pipelined_block = create_pipelined_block(block_info.clone());

        // Create an ordered block with the pipelined block and random proof
        let ordered_block = OrderedBlock::new(
            vec![pipelined_block.clone()],
            create_empty_ledger_info(current_epoch),
        );

        // Verify the ordered blocks and ensure it fails (the block IDs don't match)
        let error = ordered_block.verify_ordered_blocks().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create an ordered block proof with the same block ID
        let ordered_proof = LedgerInfoWithSignatures::new(
            LedgerInfo::new(block_info, HashValue::random()),
            AggregateSignature::empty(),
        );

        // Create an ordered block with the correct proof
        let ordered_block = OrderedBlock::new(vec![pipelined_block], ordered_proof);

        // Verify the ordered block and ensure it passes
        ordered_block.verify_ordered_blocks().unwrap();
    }

    #[test]
    fn test_verify_ordered_blocks_chained() {
        // Create multiple pipelined blocks not chained together
        let current_epoch = 0;
        let mut pipelined_blocks = vec![];
        for _ in 0..3 {
            // Create the pipelined block
            let block_id = HashValue::random();
            let block_info = create_block_info(current_epoch, block_id);
            let pipelined_block = create_pipelined_block(block_info);

            // Add the pipelined block to the list
            pipelined_blocks.push(pipelined_block);
        }

        // Create an ordered block proof with the same block ID as the last pipelined block
        let last_block_info = pipelined_blocks.last().unwrap().block_info().clone();
        let ordered_proof = LedgerInfoWithSignatures::new(
            LedgerInfo::new(last_block_info, HashValue::random()),
            AggregateSignature::empty(),
        );

        // Create an ordered block with the pipelined blocks and proof
        let ordered_block = OrderedBlock::new(pipelined_blocks, ordered_proof);

        // Verify the ordered block and ensure it fails (the blocks are not chained)
        let error = ordered_block.verify_ordered_blocks().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create multiple pipelined blocks that are chained together
        let mut pipelined_blocks = vec![];
        let mut expected_parent_id = None;
        for _ in 0..5 {
            // Create the pipelined block
            let block_id = HashValue::random();
            let block_info = create_block_info(current_epoch, block_id);
            let pipelined_block = create_pipelined_block_with_parent(
                block_info,
                expected_parent_id.unwrap_or_default(),
            );

            // Add the pipelined block to the list
            pipelined_blocks.push(pipelined_block);

            // Update the expected parent ID
            expected_parent_id = Some(block_id);
        }

        // Create an ordered block proof with the same block ID as the last pipelined block
        let last_block_info = pipelined_blocks.last().unwrap().block_info().clone();
        let ordered_proof = LedgerInfoWithSignatures::new(
            LedgerInfo::new(last_block_info, HashValue::random()),
            AggregateSignature::empty(),
        );

        // Create an ordered block with the pipelined blocks and proof
        let ordered_block = OrderedBlock::new(pipelined_blocks, ordered_proof);

        // Verify the ordered block and ensure it passes
        ordered_block.verify_ordered_blocks().unwrap();
    }

    #[test]
    fn test_verify_ordered_proof() {
        // Create a ledger info with an empty signature set
        let current_epoch = 100;
        let ledger_info = create_empty_ledger_info(current_epoch);

        // Create an epoch state for the current epoch (with an empty verifier)
        let epoch_state = EpochState::new(current_epoch, ValidatorVerifier::new(vec![]));

        // Create an ordered block message with an empty block and ordered proof
        let ordered_block = OrderedBlock::new(vec![], ledger_info);

        // Verify the ordered proof and ensure it passes
        ordered_block.verify_ordered_proof(&epoch_state).unwrap();

        // Create an epoch state for the current epoch (with a non-empty verifier)
        let validator_signer = ValidatorSigner::random(None);
        let validator_consensus_info = ValidatorConsensusInfo::new(
            validator_signer.author(),
            validator_signer.public_key(),
            100,
        );
        let validator_verifier = ValidatorVerifier::new(vec![validator_consensus_info]);
        let epoch_state = EpochState::new(current_epoch, validator_verifier);

        // Verify the ordered proof and ensure it fails (the signature set is insufficient)
        let error = ordered_block
            .verify_ordered_proof(&epoch_state)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));
    }

    #[test]
    fn test_verify_payload_digests() {
        // Create multiple signed transactions
        let num_signed_transactions = 10;
        let mut signed_transactions = create_signed_transactions(num_signed_transactions);
        let signed_transactions_for_optqs: Vec<_> = signed_transactions
            .iter()
            .cloned()
            .chain(std::iter::once(signed_transactions.last().unwrap().clone()))
            .collect();

        // Create multiple batch proofs with random digests
        let num_batches = num_signed_transactions - 1;
        let mut proofs = vec![];
        for _ in 0..num_batches {
            let batch_info = create_batch_info_with_digest(HashValue::random(), 1, 1000);
            let proof = ProofOfStore::new(batch_info, AggregateSignature::empty());
            proofs.push(proof);
        }

        // Create a single inline batch with a random digest
        let inline_batch = create_batch_info_with_digest(HashValue::random(), 1, 1000);
        let inline_batches = vec![inline_batch.clone()];

        // Create a single optqs batch with a random digest
        let opt_batch = create_batch_info_with_digest(HashValue::zero(), 1, 1000);
        let opt_and_inline_batches = vec![opt_batch, inline_batch];

        // Create a block hybrid payload (with the transactions, proofs and inline batches)
        let block_payload =
            create_block_payload(None, &signed_transactions, &proofs, &inline_batches);

        // Verify the block hybrid payload digests and ensure it fails (the batch digests don't match)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a block optqs payload (with the transactions, proofs and inline batches)
        let block_payload = create_block_optqs_payload(
            None,
            &signed_transactions_for_optqs,
            &proofs,
            &opt_and_inline_batches,
        );

        // Verify the block optqs payload digests and ensure it fails (the batch digests don't match)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create multiple batch proofs with the correct digests
        let mut proofs = vec![];
        for transaction in &signed_transactions[0..num_batches] {
            let batch_payload = BatchPayload::new(PeerId::ZERO, vec![transaction.clone()]);
            let batch_info = create_batch_info_with_digest(batch_payload.hash(), 1, 1000);
            let proof = ProofOfStore::new(batch_info, AggregateSignature::empty());
            proofs.push(proof);
        }

        // Create a block payload (with the transactions, correct proofs and inline batches)
        let block_payload =
            create_block_payload(None, &signed_transactions, &proofs, &inline_batches);

        // Verify the block payload digests and ensure it fails (the inline batch digests don't match)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a block optqs payload (with the transactions, correct proofs and optqs and inline batches)
        let block_payload = create_block_optqs_payload(
            None,
            &signed_transactions_for_optqs,
            &proofs,
            &opt_and_inline_batches,
        );

        // Verify the block optqs payload digests and ensure it fails (the inline batch digests don't match)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a single inline batch with the correct digest
        let inline_batch_payload = BatchPayload::new(PeerId::ZERO, vec![signed_transactions
            .last()
            .unwrap()
            .clone()]);
        let inline_batch_info = create_batch_info_with_digest(inline_batch_payload.hash(), 1, 1000);
        let inline_batches = vec![inline_batch_info.clone()];

        // Create a single opt batch with the correct digest
        let opt_batch_payload = BatchPayload::new(PeerId::ZERO, vec![signed_transactions
            .last()
            .unwrap()
            .clone()]);
        let opt_batch_info = create_batch_info_with_digest(opt_batch_payload.hash(), 1, 1000);
        let opt_and_inline_batches = vec![opt_batch_info, inline_batch_info];

        // Create a block payload (with the transactions, correct proofs and correct inline batches)
        let block_payload =
            create_block_payload(None, &signed_transactions, &proofs, &inline_batches);

        // Verify the block payload digests and ensure it passes
        block_payload.verify_payload_digests().unwrap();

        // Create a block payload (with the transactions, correct proofs and correct inline batches)
        let block_payload = create_block_optqs_payload(
            None,
            &signed_transactions_for_optqs,
            &proofs,
            &opt_and_inline_batches,
        );

        // Verify the block payload digests and ensure it passes
        block_payload.verify_payload_digests().unwrap();

        // Create a block payload (with too many transactions)
        signed_transactions.append(&mut create_signed_transactions(1));
        let block_payload =
            create_block_payload(None, &signed_transactions, &proofs, &inline_batches);

        // Verify the block payload digests and ensure it fails (there are too many transactions)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a block optqs payload (with too many transactions)
        let signed_transactions_for_optqs: Vec<_> = signed_transactions
            .iter()
            .cloned()
            .chain(std::iter::once(signed_transactions.last().unwrap().clone()))
            .collect();
        let block_payload = create_block_optqs_payload(
            None,
            &signed_transactions_for_optqs,
            &proofs,
            &opt_and_inline_batches,
        );

        // Verify the block payload digests and ensure it fails (there are too many transactions)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a block payload (with too few transactions)
        for _ in 0..3 {
            signed_transactions.pop();
        }
        let block_payload =
            create_block_payload(None, &signed_transactions, &proofs, &inline_batches);

        // Verify the block payload digests and ensure it fails (there are too few transactions)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a block optqs payload (with too few transactions)
        let signed_transactions_for_optqs: Vec<_> = signed_transactions
            .iter()
            .cloned()
            .chain(std::iter::once(signed_transactions.last().unwrap().clone()))
            .collect();
        let block_payload = create_block_optqs_payload(
            None,
            &signed_transactions_for_optqs,
            &proofs,
            &opt_and_inline_batches,
        );

        // Verify the block payload digests and ensure it fails (there are too few transactions)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));
    }

    #[test]
    fn test_verify_payload_digests_expired() {
        // Create a new block info with the specified timestamp
        let block_timestamp = 1000;
        let block_info = BlockInfo::new(
            0,
            0,
            HashValue::random(),
            HashValue::random(),
            0,
            block_timestamp,
            None,
        );

        // Create multiple signed transactions
        let num_signed_transactions = 100;
        let signed_transactions = create_signed_transactions(num_signed_transactions);

        // Create multiple batch proofs (where some batches are expired)
        let (proofs, non_expired_transactions) =
            create_mixed_expiration_proofs(block_timestamp, &signed_transactions);

        // Create a block payload (with non-expired transactions, all proofs and no inline batches)
        let block_payload = create_block_payload(
            Some(block_info.clone()),
            &non_expired_transactions,
            &proofs,
            &[],
        );

        // Verify the block payload digests and ensure it passes
        assert_ok!(block_payload.verify_payload_digests());

        // Create multiple inline transactions
        let num_inline_transactions = 25;
        let inline_transactions = create_signed_transactions(num_inline_transactions);

        // Create multiple inline batches (where some batches are expired)
        let (inline_batches, non_expired_inline_transactions) =
            create_mixed_expiration_proofs(block_timestamp, &inline_transactions);

        // Create a block payload (with all non-expired inline transactions, no proofs and inline batches)
        let inline_batches: Vec<_> = inline_batches
            .iter()
            .map(|proof| proof.info().clone())
            .collect();
        let block_payload = create_block_payload(
            Some(block_info.clone()),
            &non_expired_inline_transactions,
            &[],
            &inline_batches,
        );

        // Verify the block payload digests and ensure it fails (expired inline batches are still checked)
        let error = block_payload.verify_payload_digests().unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));

        // Create a block payload (with all inline transactions, no proofs and inline batches)
        let block_payload = create_block_payload(
            Some(block_info.clone()),
            &inline_transactions,
            &[],
            &inline_batches,
        );

        // Verify the block payload digests and ensure it now passes
        assert_ok!(block_payload.verify_payload_digests());

        // Gather all transactions (from both QS and inline batches)
        let all_transactions: Vec<_> = non_expired_transactions
            .iter()
            .chain(inline_transactions.iter())
            .cloned()
            .collect();

        // Create a block payload (with all transactions, all proofs and inline batches)
        let block_payload = create_block_payload(
            Some(block_info),
            &all_transactions,
            &proofs,
            &inline_batches,
        );

        // Verify the block payload digests and ensure it passes
        assert_ok!(block_payload.verify_payload_digests());
    }

    #[test]
    fn test_verify_payload_signatures() {
        // Create multiple batch info proofs (with empty signatures)
        let mut proofs = vec![];
        for _ in 0..3 {
            let batch_info = create_batch_info();
            let proof = ProofOfStore::new(batch_info, AggregateSignature::empty());
            proofs.push(proof);
        }

        // Create a transaction payload (with the proofs)
        let transaction_payload = BlockTransactionPayload::new_quorum_store_inline_hybrid(
            vec![],
            proofs.clone(),
            None,
            None,
            vec![],
            true,
        );

        // Create a block payload
        let current_epoch = 50;
        let block_info = create_block_info(current_epoch, HashValue::random());
        let block_payload = BlockPayload::new(block_info, transaction_payload);

        // Create an epoch state for the current epoch (with an empty verifier)
        let epoch_state = EpochState::new(current_epoch, ValidatorVerifier::new(vec![]));

        // Verify the block payload signatures and ensure it passes
        block_payload
            .verify_payload_signatures(&epoch_state)
            .unwrap();

        // Create an epoch state for the current epoch (with a non-empty verifier)
        let validator_signer = ValidatorSigner::random(None);
        let validator_consensus_info = ValidatorConsensusInfo::new(
            validator_signer.author(),
            validator_signer.public_key(),
            100,
        );
        let validator_verifier = ValidatorVerifier::new(vec![validator_consensus_info]);
        let epoch_state = EpochState::new(current_epoch, validator_verifier);

        // Verify the block payload signatures and ensure it fails (the signature set is insufficient)
        let error = block_payload
            .verify_payload_signatures(&epoch_state)
            .unwrap_err();
        assert_matches!(error, Error::InvalidMessageError(_));
    }

    /// Creates and returns a new batch info with random data
    fn create_batch_info() -> BatchInfo {
        create_batch_info_with_digest(HashValue::random(), 0, 0)
    }

    /// Creates and returns a new batch info with the specified digest and properties
    fn create_batch_info_with_digest(
        digest: HashValue,
        num_transactions: u64,
        batch_expiration: u64,
    ) -> BatchInfo {
        BatchInfo::new(
            PeerId::ZERO,
            BatchId::new(0),
            10,
            batch_expiration,
            digest,
            num_transactions,
            1,
            0,
        )
    }

    /// Creates and returns a new ordered block with the given block ID
    fn create_block_info(epoch: u64, block_id: HashValue) -> BlockInfo {
        BlockInfo::new(epoch, 0, block_id, HashValue::random(), 0, 0, None)
    }

    /// Creates and returns a hybrid quorum store payload using the given data
    fn create_block_payload(
        block_info: Option<BlockInfo>,
        signed_transactions: &[SignedTransaction],
        proofs: &[ProofOfStore],
        inline_batches: &[BatchInfo],
    ) -> BlockPayload {
        // Create the transaction payload
        let transaction_payload = BlockTransactionPayload::new_quorum_store_inline_hybrid(
            signed_transactions.to_vec(),
            proofs.to_vec(),
            None,
            None,
            inline_batches.to_vec(),
            true,
        );

        // Determine the block info to use
        let block_info = block_info.unwrap_or_else(|| create_block_info(0, HashValue::random()));

        // Create the block payload
        BlockPayload::new(block_info, transaction_payload)
    }

    /// Creates and returns a opt quorum store payload using the given data
    fn create_block_optqs_payload(
        block_info: Option<BlockInfo>,
        signed_transactions: &[SignedTransaction],
        proofs: &[ProofOfStore],
        opt_and_inline_batches: &[BatchInfo],
    ) -> BlockPayload {
        // Create the transaction payload
        let transaction_payload = BlockTransactionPayload::new_opt_quorum_store(
            signed_transactions.to_vec(),
            proofs.to_vec(),
            None,
            opt_and_inline_batches.to_vec(),
        );

        // Determine the block info to use
        let block_info = block_info.unwrap_or_else(|| create_block_info(0, HashValue::random()));

        // Create the block payload
        BlockPayload::new(block_info, transaction_payload)
    }

    /// Creates and returns a new ledger info with an empty signature set
    fn create_empty_ledger_info(epoch: u64) -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(BlockInfo::random_with_epoch(epoch, 0), HashValue::random()),
            AggregateSignature::empty(),
        )
    }

    /// Creates and returns a set of batch proofs using the given block
    /// timestamp and transactions. Note: some batches will be expired.
    fn create_mixed_expiration_proofs(
        block_timestamp: u64,
        signed_transactions: &[SignedTransaction],
    ) -> (Vec<ProofOfStore>, Vec<SignedTransaction>) {
        let mut proofs = vec![];
        let mut non_expired_transactions = vec![];

        // Create multiple batch proofs (each batch has 1 transaction, and some batches are expired)
        for (i, transaction) in signed_transactions.iter().enumerate() {
            // Expire every other (odd) batch and transaction
            let is_batch_expired = i % 2 != 0;

            // Determine the expiration time for the batch
            let batch_expiration = if is_batch_expired {
                block_timestamp - 1 // Older than the block timestamp
            } else {
                block_timestamp + 1 // Newer than the block timestamp
            };

            // Create and store the batch proof
            let batch_payload = BatchPayload::new(PeerId::ZERO, vec![transaction.clone()]);
            let batch_info =
                create_batch_info_with_digest(batch_payload.hash(), 1, batch_expiration);
            let proof = ProofOfStore::new(batch_info, AggregateSignature::empty());
            proofs.push(proof);

            // Save the non-expired transactions
            if !is_batch_expired {
                non_expired_transactions.push(transaction.clone());
            }
        }

        (proofs, non_expired_transactions)
    }

    /// Creates and returns a new pipelined block with the given block info
    fn create_pipelined_block(block_info: BlockInfo) -> Arc<PipelinedBlock> {
        let block_data = BlockData::new_for_testing(
            block_info.epoch(),
            block_info.round(),
            block_info.timestamp_usecs(),
            QuorumCert::dummy(),
            BlockType::Genesis,
        );
        let block = Block::new_for_testing(block_info.id(), block_data, None);
        Arc::new(PipelinedBlock::new_ordered(
            block,
            OrderedBlockWindow::empty(),
        ))
    }

    /// Creates and returns a new pipelined block with the given block info and parent ID
    fn create_pipelined_block_with_parent(
        block_info: BlockInfo,
        parent_block_id: HashValue,
    ) -> Arc<PipelinedBlock> {
        // Create the block type
        let block_type = BlockType::DAGBlock {
            author: Author::random(),
            failed_authors: vec![],
            validator_txns: vec![],
            payload: Payload::DirectMempool(vec![]),
            node_digests: vec![],
            parent_block_id,
            parents_bitvec: BitVec::with_num_bits(0),
        };

        // Create the block data
        let block_data = BlockData::new_for_testing(
            block_info.epoch(),
            block_info.round(),
            block_info.timestamp_usecs(),
            QuorumCert::dummy(),
            block_type,
        );

        // Create the pipelined block
        let block = Block::new_for_testing(block_info.id(), block_data, None);
        Arc::new(PipelinedBlock::new_ordered(
            block,
            OrderedBlockWindow::empty(),
        ))
    }

    /// Creates a returns multiple signed transactions
    fn create_signed_transactions(num_transactions: usize) -> Vec<SignedTransaction> {
        // Create a random sender and keypair
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let sender = AccountAddress::random();

        // Create multiple signed transactions
        let mut transactions = vec![];
        for i in 0..num_transactions {
            // Create the raw transaction
            // TODO[Orderless]: Change this to transaction payload v2 format
            let transaction_payload =
                TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
            let raw_transaction = RawTransaction::new(
                sender,
                i as u64,
                transaction_payload,
                0,
                0,
                0,
                ChainId::new(10),
            );

            // Create the signed transaction
            let signed_transaction = SignedTransaction::new(
                raw_transaction.clone(),
                public_key.clone(),
                private_key.sign(&raw_transaction).unwrap(),
            );

            // Save the signed transaction
            transactions.push(signed_transaction)
        }

        transactions
    }
}
