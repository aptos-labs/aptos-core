// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::{
    pipeline::commit_decision::CommitDecision, pipelined_block::PipelinedBlock,
};
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedBlock {
    pub blocks: Vec<Arc<PipelinedBlock>>,
    pub ordered_proof: LedgerInfoWithSignatures,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObserverMessage {
    OrderedBlock(OrderedBlock),
    CommitDecision(CommitDecision),
}

impl Display for ObserverMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ObserverMessage::OrderedBlock(blocks) => {
                write!(f, "OrderedBlock: {}", blocks.ordered_proof.commit_info())
            },
            ObserverMessage::CommitDecision(commit) => {
                write!(f, "CommitDecision: {}", commit.ledger_info().commit_info())
            },
        }
    }
}

impl ObserverMessage {
    pub fn epoch(&self) -> u64 {
        match self {
            ObserverMessage::OrderedBlock(blocks) => blocks.ordered_proof.commit_info().epoch(),
            ObserverMessage::CommitDecision(commit) => commit.ledger_info().commit_info().epoch(),
        }
    }
}
