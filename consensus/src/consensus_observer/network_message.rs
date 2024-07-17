// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::error::Error;
use aptos_consensus_types::pipelined_block::PipelinedBlock;
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
        transactions: Vec<SignedTransaction>,
        limit: Option<u64>,
    ) -> ConsensusObserverDirectSend {
        ConsensusObserverDirectSend::BlockPayload(BlockPayload {
            block,
            transactions,
            limit,
        })
    }
}

impl Display for ConsensusObserverMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusObserverMessage::Request(request) => {
                write!(f, "ConsensusObserverRequest: {}", request.get_content())
            },
            ConsensusObserverMessage::Response(response) => {
                write!(f, "ConsensusObserverResponse: {}", response.get_content())
            },
            ConsensusObserverMessage::DirectSend(direct_send) => {
                write!(
                    f,
                    "ConsensusObserverDirectSend: {}",
                    direct_send.get_content()
                )
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

    /// Returns the message content for the request. This is useful for debugging.
    pub fn get_content(&self) -> String {
        self.get_label().into()
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

    /// Returns the message content for the response. This is useful for debugging.
    pub fn get_content(&self) -> String {
        self.get_label().into()
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

    /// Returns the message content for the direct send. This is useful for debugging.
    pub fn get_content(&self) -> String {
        match self {
            ConsensusObserverDirectSend::OrderedBlock(ordered_block) => {
                format!("OrderedBlock: {}", ordered_block.proof_block_info())
            },
            ConsensusObserverDirectSend::CommitDecision(commit_decision) => {
                format!("CommitDecision: {}", commit_decision.proof_block_info())
            },
            ConsensusObserverDirectSend::BlockPayload(block_payload) => {
                format!(
                    "BlockPayload: {} {} {:?}",
                    block_payload.block.id(),
                    block_payload.transactions.len(),
                    block_payload.limit
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

/// Payload message contains the block, transactions and the limit of the block
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockPayload {
    pub block: BlockInfo,
    pub transactions: Vec<SignedTransaction>,
    pub limit: Option<u64>,
}
