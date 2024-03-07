// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::{
    pipeline::commit_decision::CommitDecision, pipelined_block::PipelinedBlock,
};
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
