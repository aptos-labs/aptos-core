// Copyright © Aptos Foundation

use crate::{metrics::TIMER, pipeline::ExecuteBlockMessage};
use aptos_block_partitioner::{BlockPartitioner, PartitionerConfig};
use aptos_streaming_partitioner;
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
    transaction::Transaction,
};
use std::time::Instant;
use aptos_streaming_partitioner::{SerializationIdx, StreamingTransactionPartitioner, transaction_graph_partitioner};
use aptos_streaming_partitioner::transaction_graph_partitioner::TransactionGraphPartitioner;
use aptos_transaction_orderer::transaction_compressor::compress_transactions;
use aptos_types::batched_stream::BatchedStream;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use aptos_transaction_orderer::transaction_compressor::CompressedPTransaction;

pub(crate) struct BlockPartitioningStage {
    num_executor_shards: usize,
    num_blocks_processed: usize,
    maybe_partitioner: Option<Box<dyn BlockPartitioner>>,
}

impl BlockPartitioningStage {
    pub fn new(num_shards: usize, partitioner_config: PartitionerConfig) -> Self {
        let maybe_partitioner = if num_shards <= 1 {
            None
        } else {
            let partitioner = partitioner_config.build();
            Some(partitioner)
        };

        Self {
            num_executor_shards: num_shards,
            num_blocks_processed: 0,
            maybe_partitioner,
        }
    }

    pub fn process(&mut self, mut txns: Vec<Transaction>) -> ExecuteBlockMessage {
        let current_block_start_time = Instant::now();
        let block_size = txns.len();
        info!(
            "In iteration {}, received {:?} transactions.",
            self.num_blocks_processed,
            block_size
        );
        let block_id = HashValue::random();
        // let mut params = transaction_graph_partitioner::Params {
        //     node_weight_function: |_: &CompressedPTransaction<AnalyzedTransaction>| 1 as NodeWeight,
        //     edge_weight_function,
        //     shuffle_batches: false,
        // };
        // let mut fennel = FennelGraphPartitioner::new(self.num_executor_shards);
        // fennel.balance_constraint_mode = BalanceConstraintMode::Batched;
        // fennel.alpha_computation_mode = AlphaComputationMode::Batched;
        // params.shuffle_batches = true;
        // let mut partitioner = TransactionGraphPartitioner::new(fennel, params);
        // let analyzed_transactions: Vec<AnalyzedTransaction> = txns.clone().into_iter().map(|t| t.into()).collect();
        // let compressed_transactions = compress_transactions(analyzed_transactions);
        // let batched = compressed_transactions.into_iter().batched(block_size);
        // let stream = partitioner.partition_transactions(batched).unwrap();
        // let mut txns_by_partition = vec![vec![]; self.num_executor_shards];
        // let mut partition_by_txn = vec![0; block_size];
        //
        // for batch in stream.unwrap_batches().into_no_error_batch_iter() {
        //     for tx in batch {
        //         partition_by_txn[tx.serialization_idx as usize] = tx.partition;
        //         txns_by_partition[tx.partition as usize].push(tx);
        //     }
        // }

        //TODO: wrap partition_by_txn as the block.

        let block: ExecutableBlock = match &self.maybe_partitioner {
            None => (block_id, txns).into(),
            Some(partitioner) => {
                let last_txn = txns.pop().unwrap();
                let analyzed_transactions = txns.into_iter().map(|t| t.into()).collect();
                let timer = TIMER.with_label_values(&["partition"]).start_timer();
                timer.stop_and_record();
                let mut partitioned_txns =
                    partitioner.partition(analyzed_transactions, self.num_executor_shards);
                partitioned_txns.add_checkpoint_txn(last_txn);
                ExecutableBlock::new(block_id, ExecutableTransactions::Sharded(partitioned_txns))
            },
        };
        self.num_blocks_processed += 1;
        ExecuteBlockMessage {
            current_block_start_time,
            partition_time: Instant::now().duration_since(current_block_start_time),
            block,
        }
    }
}

pub fn edge_weight_function(idx1: SerializationIdx, idx2: SerializationIdx) -> EdgeWeight {
    ((1. / (1. + idx1 as f64 - idx2 as f64)) * 100000.) as EdgeWeight
}

pub type NodeWeight = i32;
pub type EdgeWeight = i32;
