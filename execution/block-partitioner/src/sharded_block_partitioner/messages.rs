// Copyright © Aptos Foundation

use crate::{
    sharded_block_partitioner::dependency_analysis::{RWSet, WriteSetWithTxnIndex},
    types::{SubBlock, TxnIndex},
};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use std::sync::Arc;

pub enum ControlMsg {
    DiscardCrossShardDepReq(DiscardTxnsWithCrossShardDep),
    AddCrossShardDepReq(AddTxnsWithCrossShardDep),
    Stop,
}

#[derive(Clone, Debug)]
pub enum CrossShardMsg {
    WriteSetWithTxnIndexMsg(WriteSetWithTxnIndex),
    RWSetMsg(RWSet),
    // Number of accepted transactions in the shard for the current round.
    AcceptedTxnsMsg(usize),
}

pub struct DiscardTxnsWithCrossShardDep {
    pub transactions: Vec<AnalyzedTransaction>,
    // The frozen dependencies in previous chunks.
    pub prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
    pub prev_rounds_frozen_sub_blocks: Arc<Vec<SubBlock>>,
}

impl DiscardTxnsWithCrossShardDep {
    pub fn new(
        transactions: Vec<AnalyzedTransaction>,
        prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        prev_rounds_frozen_sub_blocks: Arc<Vec<SubBlock>>,
    ) -> Self {
        Self {
            transactions,
            prev_rounds_write_set_with_index,
            prev_rounds_frozen_sub_blocks,
        }
    }
}

pub struct AddTxnsWithCrossShardDep {
    pub transactions: Vec<AnalyzedTransaction>,
    pub index_offset: TxnIndex,
    // The frozen dependencies in previous chunks.
    pub prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
}

impl AddTxnsWithCrossShardDep {
    pub fn new(
        transactions: Vec<AnalyzedTransaction>,
        index_offset: TxnIndex,
        prev_rounds_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
    ) -> Self {
        Self {
            transactions,
            index_offset,
            prev_rounds_write_set_with_index,
        }
    }
}

pub struct PartitioningBlockResponse {
    pub frozen_sub_block: SubBlock,
    pub write_set_with_index: WriteSetWithTxnIndex,
    pub discarded_txns: Vec<AnalyzedTransaction>,
}

impl PartitioningBlockResponse {
    pub fn new(
        frozen_sub_block: SubBlock,
        write_set_with_index: WriteSetWithTxnIndex,
        discarded_txns: Vec<AnalyzedTransaction>,
    ) -> Self {
        Self {
            frozen_sub_block,
            write_set_with_index,
            discarded_txns,
        }
    }
}
