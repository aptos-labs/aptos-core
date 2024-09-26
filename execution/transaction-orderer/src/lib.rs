// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use std::collections::{HashMap, HashSet};
use std::io;
use std::ops::Deref;
use std::rc::Rc;
use aptos_block_partitioner::PartitionerConfig;
use aptos_block_partitioner::v3::build_partitioning_result;
use aptos_crypto::hash::CryptoHash;
use aptos_logger::prelude::*;
use aptos_types::block_executor::partitioner::{PartitionedTransactions, PartitionedTransactionsV3, PartitionV3};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use crate::batch_orderer::SequentialDynamicAriaOrderer;
use crate::block_orderer::BatchedBlockOrdererWithWindow;
use crate::block_partitioner::{BlockPartitioner, OrderedRoundRobinPartitioner};
use crate::transaction_compressor::{compress_transactions, CompressedPTransaction, CompressedPTransactionInner};

pub mod batch_orderer;
pub mod block_orderer;
pub mod block_partitioner;
pub mod common;
mod reservation_table;
pub mod transaction_compressor;

pub struct V3ReorderingPartitioner {
    pub print_debug_stats: bool,
    pub min_ordered_transaction_before_execution: usize,
    pub max_window_size: usize,
}

impl Default for V3ReorderingPartitioner {
    fn default() -> Self {
        V3ReorderingPartitioner {
            print_debug_stats: false,
            min_ordered_transaction_before_execution: 100,
            max_window_size: 1000,
        }
    }
}

impl aptos_block_partitioner::BlockPartitioner for V3ReorderingPartitioner {
    fn partition(&self, transactions: Vec<AnalyzedTransaction>, num_shards: usize) -> PartitionedTransactions {
        // for (tid, txn) in transactions.iter().enumerate() {
        //     let sender = txn.sender();
        //     let seqnum = txn.transaction().try_as_signed_user_txn().map(|t| t.sequence_number());
        //     for loc in txn.write_hints.iter() {
        //         println!("BEFORE - tid={}, sender={:?}, seq={:?}, write={}", tid, sender, seqnum, loc.state_key().hash());
        //     }
        //     for loc in txn.read_hints.iter() {
        //         println!("BEFORE - tid={}, sender={:?}, seq={:?}, read={}", tid, sender, seqnum, loc.state_key().hash());
        //     }
        // }
        let block_size = transactions.len();
        let min_ordered_transaction_before_execution = std::env::var("V3B__MIN_ORDERED_BEFORE_EXECUTION").ok().map(|v|v.parse::<usize>().unwrap_or(100)).unwrap_or(100);
        let max_window_size = std::env::var("V3B__MAX_WINDOW_SIZE").ok().map(|v|v.parse::<usize>().unwrap_or(1000)).unwrap_or(1000);
        info!("V3ReorderingPartitioner started with configs: max_window_size={}, min_ordered_transaction_before_execution={}", max_window_size, min_ordered_transaction_before_execution);

        let block_orderer = BatchedBlockOrdererWithWindow::new(
            SequentialDynamicAriaOrderer::with_window(),
            min_ordered_transaction_before_execution * 5,
            max_window_size,
        );
        let block_partitioner = OrderedRoundRobinPartitioner::new(
            block_orderer,
            num_shards,
            (min_ordered_transaction_before_execution + num_shards - 1) / num_shards,
        );
        let (transactions, compressor) = compress_transactions(transactions);

        let mut txns_in_new_order: Vec<Option<CompressedPTransaction<AnalyzedTransaction>>> = vec![None; block_size];
        let mut shard_idxs: Vec<usize> = vec![0; block_size];
        block_partitioner
            .partition_transactions(transactions, |sharded_txns| -> Result<(), io::Error> {
                for (shard_idx, txns) in sharded_txns.into_iter().enumerate() {
                    // for (tid, compressed_t) in txns.iter() {
                    //     // let t = Rc::try_unwrap(compressed_t).unwrap();
                    //     // let CompressedPTransactionInner{ original, read_set, write_set } = t;
                    //     // let analyzed_txn = *original;
                    //     // let original_t = &compressed_t.original;
                    //     // let sender = original_t.sender();
                    //     // let seqnum = original_t.transaction().try_as_signed_user_txn().map(|t|t.sequence_number());
                    //     // for loc in original_t.write_hints.iter() {
                    //     //     println!("AFTER - shard_id={}, tid={}, sender={:?}, seq={:?}, write={}", shard_idx, tid, sender, seqnum, loc.state_key().hash());
                    //     // }
                    //     // for x in original_t.read_hints.iter() {
                    //     //     println!("AFTER - shard_id={}, tid={}, sender={:?}, seq={:?}, read={}", shard_idx, tid, sender, seqnum, x.state_key().hash());
                    //     // }
                    // }
                    for (idx, txn) in txns {
                        txns_in_new_order[idx] = Some(txn);
                        shard_idxs[idx] = shard_idx;
                    }
                }
                // count_ordered += sharded_txns.iter().map(|txns| txns.len()).sum::<usize>();
                // if latency.is_none() && count_ordered >= min_ordered_transaction_before_execution {
                //     latency = Some(now.elapsed());
                // }
                // println!("Partitioned {} transactions ({} new)", count_ordered,
                //          sharded_txns.iter().map(|txns| txns.len()).sum::<usize>());
                Ok(())
            })
            .unwrap();

        drop(block_partitioner);

        let txns = txns_in_new_order.into_iter()
            .map(|t|{
                let CompressedPTransactionInner{ original, .. } = Rc::try_unwrap(t.unwrap()).unwrap();
                *original
            })
            .collect();

        PartitionedTransactions::V3(build_partitioning_result(num_shards, txns, shard_idxs, self.print_debug_stats))
    }
}

#[derive(Debug, Default)]
pub struct V3ReorderingPartitionerConfig {}

impl PartitionerConfig for V3ReorderingPartitionerConfig {
    fn build(&self) -> Box<dyn aptos_block_partitioner::BlockPartitioner> {
        Box::new(V3ReorderingPartitioner::default())
    }
}
