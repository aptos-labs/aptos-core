// Copyright © Aptos Foundation

pub mod transaction_graph_partitioner;

use aptos_graphs::partitioning::PartitionId;
use aptos_transaction_orderer::common::PTransaction;
use aptos_types::batched_stream::{Batched, BatchedStream};
use std::collections::{BTreeSet, HashMap};
use aptos_block_partitioner::BlockPartitioner;
use aptos_graphs::graph::{EdgeWeight, NodeWeight};
use aptos_graphs::partitioning::fennel::{AlphaComputationMode, BalanceConstraintMode, FennelGraphPartitioner};
use aptos_transaction_orderer::transaction_compressor::{compress_transactions, CompressedPTransaction, CompressedPTransactionInner};
use aptos_types::block_executor::partitioner::PartitionedTransactions;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use std::rc::Rc;
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


pub struct PartitionerV3 {
}

impl BlockPartitioner for PartitionerV3 {
    fn partition(&self, block_id: u8, transactions: Vec<AnalyzedTransaction>, num_shards: usize) -> PartitionedTransactions {
        let block_size = transactions.len();
        let mut fennel = FennelGraphPartitioner::new(num_shards);
        fennel.balance_constraint_mode = BalanceConstraintMode::Batched;
        fennel.alpha_computation_mode = AlphaComputationMode::Batched;
        let params = transaction_graph_partitioner::Params {
            node_weight_function: |_: &CompressedPTransaction<AnalyzedTransaction>| 1 as NodeWeight,
            edge_weight_function: |idx1: SerializationIdx, idx2: SerializationIdx| ((1. / (1. + idx1 as f64 - idx2 as f64)) * 1000000.) as EdgeWeight,
            shuffle_batches: true,
        };
        let mut partitioner = TransactionGraphPartitioner::new(fennel, params);
        let compressed_txns = compress_transactions(transactions);
        let transactions = compressed_txns.into_iter().batched(block_size);

        let mut stream = partitioner.partition_transactions(transactions).unwrap();
        let mut global_idxs: Vec<Vec<SerializationIdx>> = vec![vec![]; num_shards];
        let mut txns = vec![None; block_size];
        let mut shard_idxs_by_txn: Vec<usize> = vec![0; block_size];
        let mut dependency_sets = vec![vec![]; block_size];
        let mut follower_sets = vec![vec![]; block_size];
        for batch in stream.unwrap_batches().into_no_error_batch_iter() {
            for tx in batch {
                let PartitionedTransaction{ transaction, serialization_idx, partition, dependencies } = tx;
                let t = Rc::try_unwrap(transaction).unwrap();
                let CompressedPTransactionInner{ original, read_set, write_set } = t;
                let analyzed_txn = *original;
                shard_idxs_by_txn[serialization_idx as usize] = partition as usize;
                for &src_idx in dependencies.keys() {
                    dependency_sets[serialization_idx as usize].push(src_idx);
                    follower_sets[src_idx as usize].push(serialization_idx)
                }
                global_idxs[partition as usize].push(serialization_idx);
                txns[serialization_idx as usize] = Some(analyzed_txn);
            }
        }

        // Ensure txns and indices are sorted.
        let sharded_txns = global_idxs.iter_mut().map(|idxs| {
            idxs.sort();
            let txns: Vec<AnalyzedTransaction> = idxs.iter().map(|idx|txns[*idx as usize].take().unwrap()).collect();
            txns
        }).collect();

        PartitionedTransactions {
            block_id,
            sharded_txns,
            global_idxs,
            shard_idxs_by_txn,
            dependency_sets,
            follower_sets,
        }
    }
}
