// Copyright © Aptos Foundation

use crate::sharded_block_partitioner::dependency_analysis::WriteSetWithTxnIndex;
use aptos_types::{
    block_executor::partitioner::{RoundId, SubBlocksForShard, TxnIndex},
    transaction::{analyzed_transaction::AnalyzedTransaction, Transaction},
};
use std::sync::Arc;

pub struct DiscardCrossShardDep {
    pub transactions: Vec<AnalyzedTransaction>,
    pub round_id: RoundId,
}

pub struct AddWithCrossShardDep {
    pub transactions: Vec<AnalyzedTransaction>,
    pub index_offset: TxnIndex,
    // The frozen dependencies in previous chunks.
    pub prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
    pub frozen_sub_blocks: SubBlocksForShard<Transaction>,
    pub round_id: RoundId,
}

impl AddWithCrossShardDep {
    pub fn new(
        transactions: Vec<AnalyzedTransaction>,
        index_offset: TxnIndex,
        prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        frozen_sub_blocks: SubBlocksForShard<Transaction>,
        round_id: RoundId,
    ) -> Self {
        Self {
            transactions,
            index_offset,
            prev_rounds_write_set_with_index,
            frozen_sub_blocks,
            round_id,
        }
    }
}

pub struct PartitioningResp {
    pub accepted_txns: Vec<AnalyzedTransaction>,
    pub discarded_txns: Vec<AnalyzedTransaction>,
}

pub enum ControlMsg {
    DiscardCrossShardDepReq(DiscardCrossShardDep),
    Stop,
}
