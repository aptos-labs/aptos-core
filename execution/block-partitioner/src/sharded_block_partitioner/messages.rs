// Copyright © Aptos Foundation

use crate::{
    sharded_block_partitioner::dependency_analysis::{RWSet, RWSetWithTxnIndex},
    types::{TransactionsChunk, TxnIndex},
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
    RWSetWithTxnIndexMsg(RWSetWithTxnIndex),
    RWSetMsg(RWSet),
    // Number of accepted transactions in the shard for the current round.
    AcceptedTxnsMsg(usize),
}

pub struct DiscardTxnsWithCrossShardDep {
    pub transactions: Vec<AnalyzedTransaction>,
    // The frozen dependencies in previous chunks.
    pub prev_rounds_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
    pub prev_rounds_frozen_chunks: Arc<Vec<TransactionsChunk>>,
}

impl DiscardTxnsWithCrossShardDep {
    pub fn new(
        transactions: Vec<AnalyzedTransaction>,
        prev_rounds_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
        prev_rounds_frozen_chunks: Arc<Vec<TransactionsChunk>>,
    ) -> Self {
        Self {
            transactions,
            prev_rounds_rw_set_with_index,
            prev_rounds_frozen_chunks,
        }
    }
}

pub struct AddTxnsWithCrossShardDep {
    pub transactions: Vec<AnalyzedTransaction>,
    pub index_offset: TxnIndex,
    pub prev_rounds_frozen_chunks: Arc<Vec<TransactionsChunk>>,
    // The frozen dependencies in previous chunks.
    pub prev_rounds_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
}

impl AddTxnsWithCrossShardDep {
    pub fn new(
        transactions: Vec<AnalyzedTransaction>,
        index_offset: TxnIndex,
        prev_rounds_frozen_chunks: Arc<Vec<TransactionsChunk>>,
        prev_rounds_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
    ) -> Self {
        Self {
            transactions,
            index_offset,
            prev_rounds_rw_set_with_index,
            prev_rounds_frozen_chunks,
        }
    }
}

pub struct PartitioningBlockResponse {
    pub frozen_chunk: TransactionsChunk,
    pub rw_set_with_index: RWSetWithTxnIndex,
    pub rejected_txns: Vec<AnalyzedTransaction>,
}

impl PartitioningBlockResponse {
    pub fn new(
        frozen_chunk: TransactionsChunk,
        frozen_dependencies: RWSetWithTxnIndex,
        rejected_txns: Vec<AnalyzedTransaction>,
    ) -> Self {
        Self {
            frozen_chunk,
            rw_set_with_index: frozen_dependencies,
            rejected_txns,
        }
    }
}
