// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

pub mod transaction_graph_partitioner;

use aptos_graphs::partitioning::PartitionId;
use aptos_logger::prelude::*;
use aptos_transaction_orderer::common::PTransaction;
use aptos_types::batched_stream::{Batched, BatchedStream};
use std::collections::{BTreeSet, HashMap, HashSet};
use aptos_block_partitioner::{BlockPartitioner, PartitionerConfig};
use aptos_graphs::graph::{EdgeWeight, NodeWeight};
use aptos_graphs::partitioning::fennel::{AlphaComputationMode, BalanceConstraintMode, FennelGraphPartitioner};
use aptos_transaction_orderer::transaction_compressor::{compress_transactions, CompressedPTransaction, CompressedPTransactionInner};
use aptos_types::block_executor::partitioner::PartitionedTransactions;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use std::rc::Rc;
use aptos_block_partitioner::v3::build_partitioning_result;
use aptos_types::state_store::state_key::StateKey;
use crate::transaction_graph_partitioner::TransactionGraphPartitioner;

/// Indicates the position of the transaction in the serialization order of the block.
pub type SerializationIdx = u32;

/// A transaction with its dependencies, serialization index, and partition.
#[derive(Clone, Debug)]
pub struct PartitionedTransaction<T: PTransaction> {
    pub transaction: T,
    pub serialization_idx: SerializationIdx,
    pub partition: PartitionId,
    pub dependencies: HashMap<SerializationIdx, Vec<T::Key>>,
}

/// A trait for streaming transaction partitioners.
pub trait StreamingTransactionPartitioner<S>
where
    S: BatchedStream,
    S::StreamItem: PTransaction,
{
    /// The error type returned by the partitioner.
    type Error;

    type ResultStream: BatchedStream<
        StreamItem = PartitionedTransaction<S::StreamItem>,
        Error = Self::Error,
    >;

    fn partition_transactions(
        &mut self,
        transactions: S,
    ) -> Result<Self::ResultStream, Self::Error>;
}

#[derive(Default)]
pub struct V3FennelBasedPartitioner {
    pub print_debug_stats: bool,
}

impl BlockPartitioner for V3FennelBasedPartitioner {
    fn partition(&self, transactions: Vec<AnalyzedTransaction>, num_shards: usize) -> PartitionedTransactions {
        info!("V3FennelBasedPartitioner started.");
        let block_size = transactions.len();
        let mut fennel = FennelGraphPartitioner::new(num_shards);
        fennel.balance_constraint_mode = BalanceConstraintMode::Batched;
        fennel.alpha_computation_mode = AlphaComputationMode::Batched;
        let params = transaction_graph_partitioner::Params {
            node_weight_function: |_: &CompressedPTransaction<AnalyzedTransaction>| 1 as NodeWeight,
            edge_weight_function: |idx1: SerializationIdx, idx2: SerializationIdx| ((1. / (1. + idx1 as f64 - idx2 as f64)) * 1000000.) as EdgeWeight,//sharding v3 todo: tweak edge_weight_function
            shuffle_batches: true,
        };
        let mut partitioner = TransactionGraphPartitioner::new(fennel, params);
        let (compressed_txns, compressor) = compress_transactions(transactions);
        let transactions = compressed_txns.into_iter().batched(block_size);

        let stream = partitioner.partition_transactions(transactions).unwrap();
        let mut txn_holders = vec![None; block_size];
        let mut shard_idxs_by_txn: Vec<usize> = vec![0; block_size];
        for batch in stream.unwrap_batches().into_no_error_batch_iter() {
            for txn in batch {
                let PartitionedTransaction{ transaction, serialization_idx, partition, .. } = txn;
                let CompressedPTransactionInner{ original, .. } = Rc::try_unwrap(transaction).unwrap();
                let analyzed_txn = *original;
                shard_idxs_by_txn[serialization_idx as usize] = partition as usize;
                txn_holders[serialization_idx as usize] = Some(analyzed_txn);
            }
        }

        let txns = txn_holders.into_iter().map(|holder| holder.unwrap()).collect();
        PartitionedTransactions::V3(build_partitioning_result(num_shards,txns, shard_idxs_by_txn, self.print_debug_stats, false))
    }
}

#[derive(Debug, Default)]
pub struct V3FennelBasedPartitionerConfig {}

impl PartitionerConfig for V3FennelBasedPartitionerConfig {
    fn build(&self) -> Box<dyn BlockPartitioner> {
        Box::new(V3FennelBasedPartitioner::default())
    }
}
