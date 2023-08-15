// Copyright © Aptos Foundation

use crate::sharded_block_partitioner::dependency_analysis::WriteSetWithTxnIndex;
use aptos_types::{
    block_executor::partitioner::{RoundId, SubBlocksForShard, TxnIndex},
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use std::sync::Arc;

pub struct DiscardCrossShardDep {
    pub transactions: Vec<AnalyzedTransaction>,
    // The frozen dependencies in previous chunks.
    pub prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
    pub current_round_start_index: TxnIndex,
    // This is the frozen sub block for the current shard and is passed because we want to modify
    // it to add dependency back edges.
    pub frozen_sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
    pub round_id: RoundId,
}

impl DiscardCrossShardDep {
    pub fn new(
        transactions: Vec<AnalyzedTransaction>,
        prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        current_round_start_index: TxnIndex,
        frozen_sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
        round_id: RoundId,
    ) -> Self {
        Self {
            transactions,
            prev_rounds_write_set_with_index,
            current_round_start_index,
            frozen_sub_blocks,
            round_id,
        }
    }
}

pub struct AddWithCrossShardDep {
    pub transactions: Vec<AnalyzedTransaction>,
    pub index_offset: TxnIndex,
    // The frozen dependencies in previous chunks.
    pub prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
    pub frozen_sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
    pub round_id: RoundId,
}

impl AddWithCrossShardDep {
    pub fn new(
        transactions: Vec<AnalyzedTransaction>,
        index_offset: TxnIndex,
        prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        frozen_sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
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
    pub frozen_sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
    pub write_set_with_index: WriteSetWithTxnIndex,
    pub discarded_txns: Vec<AnalyzedTransaction>,
}

impl PartitioningResp {
    pub fn new(
        frozen_sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
        write_set_with_index: WriteSetWithTxnIndex,
        discarded_txns: Vec<AnalyzedTransaction>,
    ) -> Self {
        Self {
            frozen_sub_blocks,
            write_set_with_index,
            discarded_txns,
        }
    }
}

pub enum ControlMsg {
    DiscardCrossShardDepReq(DiscardCrossShardDep),
    AddCrossShardDepReq(AddWithCrossShardDep),
    Stop,
}
