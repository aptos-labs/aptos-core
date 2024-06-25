// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::{
    pipeline::commit_decision::CommitDecision, pipelined_block::PipelinedBlock,
};
use aptos_types::{
    block_info::BlockInfo, ledger_info::LedgerInfoWithSignatures, transaction::SignedTransaction,
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
        commit_decision: CommitDecision,
    ) -> ConsensusObserverDirectSend {
        ConsensusObserverDirectSend::CommitDecision(commit_decision)
    }

    /// Creates and returns a new block payload message using the given block, transactions and limit
    pub fn new_block_payload_message(
        block: BlockInfo,
        transactions: Vec<SignedTransaction>,
        limit: Option<usize>,
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
                format!(
                    "OrderedBlock: {}",
                    ordered_block.ordered_proof.commit_info()
                )
            },
            ConsensusObserverDirectSend::CommitDecision(commit_decision) => {
                format!(
                    "CommitDecision: {}",
                    commit_decision.ledger_info().commit_info()
                )
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
    pub blocks: Vec<Arc<PipelinedBlock>>,
    pub ordered_proof: LedgerInfoWithSignatures,
}

/// Payload message contains the block, transactions and the limit of the block
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockPayload {
    pub block: BlockInfo,
    pub transactions: Vec<SignedTransaction>,
    pub limit: Option<usize>,
}
