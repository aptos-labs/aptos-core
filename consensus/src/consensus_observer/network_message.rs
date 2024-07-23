// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::error::Error;
use aptos_consensus_types::{
    common::{BatchPayload, Payload},
    pipelined_block::PipelinedBlock,
    proof_of_store::{BatchInfo, ProofCache, ProofOfStore},
};
use aptos_crypto::hash::CryptoHash;
use aptos_types::{
    block_info::{BlockInfo, Round},
    epoch_change::Verifier,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    transaction::SignedTransaction,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    slice::Iter,
    sync::Arc,
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
        ConsensusObserverDirectSend::OrderedBlock(OrderedBlock {
            blocks,
            ordered_proof,
        })
    }

    /// Creates and returns a new commit decision message using the given commit decision
    pub fn new_commit_decision_message(
        commit_proof: LedgerInfoWithSignatures,
    ) -> ConsensusObserverDirectSend {
        ConsensusObserverDirectSend::CommitDecision(CommitDecision { commit_proof })
    }

    /// Creates and returns a new block payload message using the given block, transactions and limit
    pub fn new_block_payload_message(
        block: BlockInfo,
        transaction_payload: BlockTransactionPayload,
    ) -> ConsensusObserverDirectSend {
        ConsensusObserverDirectSend::BlockPayload(BlockPayload {
            block,
            transaction_payload,
        })
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
}

impl ConsensusObserverDirectSend {
    /// Returns a summary label for the direct send
    pub fn get_label(&self) -> &'static str {
        match self {
            ConsensusObserverDirectSend::OrderedBlock(_) => "ordered_block",
            ConsensusObserverDirectSend::CommitDecision(_) => "commit_decision",
            ConsensusObserverDirectSend::BlockPayload(_) => "block_payload",
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
                    block_payload.transaction_payload.limit(),
                    block_payload.transaction_payload.payload_proofs(),
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
    pub transactions: Vec<SignedTransaction>,
    pub proofs: Vec<ProofOfStore>,
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
    pub payload_with_proof: PayloadWithProof,
    pub transaction_limit: Option<u64>,
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

/// The transaction payload of each block
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BlockTransactionPayload {
    InQuorumStore(PayloadWithProof),
    InQuorumStoreWithLimit(PayloadWithProofAndLimit),
    QuorumStoreInlineHybrid(PayloadWithProofAndLimit, Vec<BatchInfo>),
    OptQuorumStore(PayloadWithProofAndLimit, Vec<BatchInfo>),
}

impl BlockTransactionPayload {
    /// Creates a returns a new InQuorumStore transaction payload
    pub fn new_in_quorum_store(
        transactions: Vec<SignedTransaction>,
        proofs: Vec<ProofOfStore>,
    ) -> Self {
        let payload_with_proof = PayloadWithProof::new(transactions, proofs);
        Self::InQuorumStore(payload_with_proof)
    }

    /// Creates a returns a new InQuorumStoreWithLimit transaction payload
    pub fn new_in_quorum_store_with_limit(
        transactions: Vec<SignedTransaction>,
        proofs: Vec<ProofOfStore>,
        limit: Option<u64>,
    ) -> Self {
        let payload_with_proof = PayloadWithProof::new(transactions, proofs);
        let proof_with_limit = PayloadWithProofAndLimit::new(payload_with_proof, limit);
        Self::InQuorumStoreWithLimit(proof_with_limit)
    }

    /// Creates a returns a new QuorumStoreInlineHybrid transaction payload
    pub fn new_quorum_store_inline_hybrid(
        transactions: Vec<SignedTransaction>,
        proofs: Vec<ProofOfStore>,
        limit: Option<u64>,
        inline_batches: Vec<BatchInfo>,
    ) -> Self {
        let payload_with_proof = PayloadWithProof::new(transactions, proofs);
        let proof_with_limit = PayloadWithProofAndLimit::new(payload_with_proof, limit);
        Self::QuorumStoreInlineHybrid(proof_with_limit, inline_batches)
    }

    pub fn new_opt_quorum_store(
        transactions: Vec<SignedTransaction>,
        proofs: Vec<ProofOfStore>,
        limit: Option<u64>,
        batch_infos: Vec<BatchInfo>,
    ) -> Self {
        let payload_with_proof = PayloadWithProof::new(transactions, proofs);
        let proof_with_limit = PayloadWithProofAndLimit::new(payload_with_proof, limit);
        Self::OptQuorumStore(proof_with_limit, batch_infos)
    }

    #[cfg(test)]
    /// Returns an empty transaction payload (for testing)
    pub fn empty() -> Self {
        Self::QuorumStoreInlineHybrid(PayloadWithProofAndLimit::empty(), vec![])
    }

    /// Returns the list of inline batches in the transaction payload
    pub fn inline_batches(&self) -> Vec<&BatchInfo> {
        match self {
            BlockTransactionPayload::QuorumStoreInlineHybrid(_, inline_batches) => {
                inline_batches.iter().collect()
            },
            _ => vec![],
        }
    }

    /// Returns the limit of the transaction payload
    pub fn limit(&self) -> Option<u64> {
        match self {
            BlockTransactionPayload::InQuorumStore(_) => None,
            BlockTransactionPayload::InQuorumStoreWithLimit(payload) => payload.transaction_limit,
            BlockTransactionPayload::QuorumStoreInlineHybrid(payload, _) => {
                payload.transaction_limit
            },
            BlockTransactionPayload::OptQuorumStore(payload, _) => payload.transaction_limit,
        }
    }

    /// Returns the proofs of the transaction payload
    pub fn payload_proofs(&self) -> Vec<ProofOfStore> {
        match self {
            BlockTransactionPayload::InQuorumStore(payload) => payload.proofs.clone(),
            BlockTransactionPayload::InQuorumStoreWithLimit(payload) => {
                payload.payload_with_proof.proofs.clone()
            },
            BlockTransactionPayload::QuorumStoreInlineHybrid(payload, _) => {
                payload.payload_with_proof.proofs.clone()
            },
            BlockTransactionPayload::OptQuorumStore(payload, _) => {
                payload.payload_with_proof.proofs.clone()
            },
        }
    }

    /// Returns the transactions in the payload
    pub fn transactions(&self) -> Vec<SignedTransaction> {
        match self {
            BlockTransactionPayload::InQuorumStore(payload) => payload.transactions.clone(),
            BlockTransactionPayload::InQuorumStoreWithLimit(payload) => {
                payload.payload_with_proof.transactions.clone()
            },
            BlockTransactionPayload::QuorumStoreInlineHybrid(payload, _) => {
                payload.payload_with_proof.transactions.clone()
            },
            BlockTransactionPayload::OptQuorumStore(payload, _) => {
                payload.payload_with_proof.transactions.clone()
            },
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
            Payload::OptQuorumStore(opt_qs_payload) => {
                // Verify the batches in the requested block
                self.verify_batches(opt_qs_payload.proof_with_data())?;

                // Verify the inline batches
                self.verify_opt_batches(opt_qs_payload.opt_batches())?;

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
            BlockTransactionPayload::QuorumStoreInlineHybrid(_, inline_batches) => {
                inline_batches.iter().map(|batch_info| batch_info).collect()
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

    fn verify_opt_batches(&self, expected_opt_batches: &Vec<BatchInfo>) -> Result<(), Error> {
        let opt_batches: &Vec<BatchInfo> = match self {
            BlockTransactionPayload::OptQuorumStore(_, opt_batches) => opt_batches,
            _ => {
                return Err(Error::InvalidMessageError(
                    "Transaction payload is not an OptQS Payload".to_string(),
                ))
            },
        };

        if expected_opt_batches != opt_batches {
            return Err(Error::InvalidMessageError(format!(
                "Transaction payload failed optimistic batch verification! Expected optimistic batches {:?} but found {:?}",
                expected_opt_batches, opt_batches
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
            BlockTransactionPayload::InQuorumStoreWithLimit(payload) => payload.transaction_limit,
            BlockTransactionPayload::QuorumStoreInlineHybrid(payload, _) => {
                payload.transaction_limit
            },
            _ => {
                return Err(Error::InvalidMessageError(
                    "Transaction payload does not contain a limit!".to_string(),
                ))
            },
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
    pub block: BlockInfo,
    pub transaction_payload: BlockTransactionPayload,
}

impl BlockPayload {
    pub fn new(block: BlockInfo, transaction_payload: BlockTransactionPayload) -> Self {
        Self {
            block,
            transaction_payload,
        }
    }

    /// Verifies the block payload digests and returns an error if the data is invalid
    pub fn verify_payload_digests(&self) -> Result<(), Error> {
        // Verify the proof of store digests against the transaction
        let transactions = self.transaction_payload.transactions();
        let mut transactions_iter = transactions.iter();
        for proof_of_store in &self.transaction_payload.payload_proofs() {
            reconstruct_and_verify_batch(&mut transactions_iter, proof_of_store.info())?;
        }

        // Verify the inline batch digests against the inline batches
        for batch_info in self.transaction_payload.inline_batches() {
            reconstruct_and_verify_batch(&mut transactions_iter, batch_info)?;
        }

        // Verify that there are no transactions remaining
        if transactions_iter.next().is_some() {
            return Err(Error::InvalidMessageError(format!(
                "Failed to verify payload transactions! Transactions remaining: {:?}. Expected: 0",
                transactions_iter.as_slice().len()
            )));
        }

        Ok(()) // All digests match
    }

    /// Verifies that the block payload proofs are correctly signed according
    /// to the current epoch state. Returns an error if the data is invalid.
    pub fn verify_payload_signatures(&self, epoch_state: &EpochState) -> Result<(), Error> {
        // Create a dummy proof cache to verify the proofs
        let proof_cache = ProofCache::new(1);

        // TODO: parallelize the verification of the proof signatures!

        // Verify each of the proof signatures
        let validator_verifier = &epoch_state.verifier;
        for proof_of_store in &self.transaction_payload.payload_proofs() {
            if let Err(error) = proof_of_store.verify(validator_verifier, &proof_cache) {
                return Err(Error::InvalidMessageError(format!(
                    "Failed to verify the proof of store for batch: {:?}, Error: {:?}",
                    proof_of_store.info(),
                    error
                )));
            }
        }

        Ok(()) // All proofs are correctly signed
    }
}

/// Reconstructs and verifies the batch using the
/// given transactions and the expected batch info.
fn reconstruct_and_verify_batch(
    transactions_iter: &mut Iter<SignedTransaction>,
    expected_batch_info: &BatchInfo,
) -> Result<(), Error> {
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
        batch_transactions.push(batch_transaction.clone());
    }

    // Calculate the batch digest
    let batch_payload = BatchPayload::new(expected_batch_info.author(), batch_transactions);
    let batch_digest = batch_payload.hash();

    // Verify the reconstructed digest against the expected digest
    let expected_digest = expected_batch_info.digest();
    if batch_digest != *expected_digest {
        return Err(Error::InvalidMessageError(format!(
            "The reconstructed inline batch digest does not match the expected digest!\
             Batch: {:?}, Expected digest: {:?}, Reconstructed digest: {:?}",
            expected_batch_info, expected_digest, batch_digest
        )));
    }

    Ok(())
}
