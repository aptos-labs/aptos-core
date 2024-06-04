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

/// Types of messages that can be sent by consensus observer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusObserverMessage {
    OrderedBlock(OrderedBlock),
    CommitDecision(CommitDecision),
    Payload((BlockInfo, (Vec<SignedTransaction>, Option<usize>))),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedBlock {
    pub blocks: Vec<Arc<PipelinedBlock>>,
    pub ordered_proof: LedgerInfoWithSignatures,
}

impl Display for ConsensusObserverMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusObserverMessage::OrderedBlock(blocks) => {
                write!(f, "OrderedBlock: {}", blocks.ordered_proof.commit_info())
            },
            ConsensusObserverMessage::CommitDecision(commit) => {
                write!(f, "CommitDecision: {}", commit.ledger_info().commit_info())
            },
            ConsensusObserverMessage::Payload((block, (payload, limit))) => {
                write!(f, "Payload: {} {} {:?}", block.id(), payload.len(), limit)
            },
        }
    }
}

impl ConsensusObserverMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            ConsensusObserverMessage::OrderedBlock(blocks) => blocks.ordered_proof.commit_info().epoch(),
            ConsensusObserverMessage::CommitDecision(commit) => commit.ledger_info().commit_info().epoch(),
            ConsensusObserverMessage::Payload((block, _)) => block.epoch(),
        }
    }
}
