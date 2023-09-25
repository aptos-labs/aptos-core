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
use aptos_crypto::hash::CryptoHash;
use aptos_streaming_partitioner::{PartitionerV3, SerializationIdx, StreamingTransactionPartitioner, transaction_graph_partitioner};
use aptos_streaming_partitioner::transaction_graph_partitioner::TransactionGraphPartitioner;
use aptos_transaction_orderer::PartitionerV3B;
use aptos_types::batched_stream::BatchedStream;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use aptos_transaction_orderer::transaction_compressor::CompressedPTransaction;
use aptos_types::block_executor::partitioner::PartitionedTransactions;
use move_core_types::account_address::AccountAddress;

pub(crate) struct BlockPartitioningStage {
    num_executor_shards: usize,
    num_blocks_processed: usize,
    partitioner: Option<Box<dyn BlockPartitioner>>,
}

impl BlockPartitioningStage {
    pub fn new(num_shards: usize, _partitioner_config: bool) -> Self {
        Self {
            num_executor_shards: num_shards,
            num_blocks_processed: 0,
            partitioner: if _partitioner_config {
                match std::env::var("V3B") {
                    Ok(v) if v.as_str() == "1" => {
                        Some(Box::new(PartitionerV3B{}))
                    },
                    _ => {
                        Some(Box::new(PartitionerV3{}))
                    }
                }
            } else {
                None
            },
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
        let block = match &self.partitioner {
            None => {
                ExecutableBlock::new(block_id, ExecutableTransactions::Unsharded(txns))
            }
            Some(partitioner) => {
                let analyzed_txns: Vec<AnalyzedTransaction> = txns.into_iter().map(AnalyzedTransaction::from).collect();
                // //debugging stuff
                // for (rank, txn) in analyzed_txns.iter().enumerate() {
                //     println!("[BeforePartitioner] block={}, rank={}, txn={:?}", block_id_short, rank, get_account_seq_number(txn.transaction()));
                // }
                let timer = TIMER.with_label_values(&["partition"]).start_timer();
                let partitioned_txns = partitioner.partition(block_id.as_ref().clone(), analyzed_txns, self.num_executor_shards);
                timer.stop_and_record();
                //debugging stuff
                // println!("block={}, global_idxs={:?}", block_id_short, partitioned_txns.global_idxs);
                // for (txn_idx, dep_set) in partitioned_txns.dependency_sets.iter().enumerate() {
                //     println!("block={}, partitioned_txns.dependency_sets[{}]={:?}", block_id_short, txn_idx, dep_set.keys().copied());
                // }
                // let txns = PartitionedTransactions::flatten(partitioned_txns.clone());
                // for (rank, txn) in txns.iter().enumerate() {
                //     println!("[AfterPartitioner] block={}, rank={}, txn={:?}", block_id_short, rank, get_account_seq_number(txn.transaction()));
                // }
                // for (txn_idx, txn) in txns.into_iter().enumerate() {
                //     for x in txn.write_hints() {
                //         println!("block={}, rank={}, txn={:?}, key={}", block_id_short, txn_idx, get_account_seq_number(txn.transaction()), x.state_key().hash().to_hex());
                //     }
                // }
                ExecutableBlock::new(block_id, ExecutableTransactions::Sharded(partitioned_txns))
            }
        };
        self.num_blocks_processed += 1;
        ExecuteBlockMessage {
            current_block_start_time,
            partition_time: Instant::now().duration_since(current_block_start_time),
            block,
        }
    }
}

fn get_account_seq_number(txn: &Transaction) -> Option<(AccountAddress, u64)> {
    match txn {
        Transaction::UserTransaction(txn) => Some((txn.sender(), txn.sequence_number())),
        _ => None,
    }
}
