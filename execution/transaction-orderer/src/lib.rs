// Copyright © Aptos Foundation

use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::io;
use std::ops::Deref;
use std::rc::Rc;
use aptos_crypto::hash::CryptoHash;
use aptos_types::block_executor::partitioner::PartitionedTransactions;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use crate::batch_orderer::SequentialDynamicAriaOrderer;
use crate::block_orderer::BatchedBlockOrdererWithWindow;
use crate::block_partitioner::{BlockPartitioner, OrderedRoundRobinPartitioner};
use crate::transaction_compressor::{compress_transactions, CompressedKey, CompressedPTransaction, CompressedPTransactionInner};

pub mod batch_orderer;
pub mod block_orderer;
pub mod block_partitioner;
pub mod common;
mod reservation_table;
pub mod transaction_compressor;

pub struct PartitionerV3B {}

impl aptos_block_partitioner::BlockPartitioner for PartitionerV3B {
    fn partition(&self, block_id: [u8; 32], transactions: Vec<AnalyzedTransaction>, num_shards: usize) -> PartitionedTransactions {
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
        println!("V3B configs: max_window_size={}, min_ordered_transaction_before_execution={}", max_window_size, min_ordered_transaction_before_execution);

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

        let mut partition_result: Vec<Vec<(usize, CompressedPTransaction<AnalyzedTransaction>)>> = vec![vec![]; num_shards];
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
                    partition_result[shard_idx].extend(txns);
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

        let mut compressed_txns_by_global_idx: Vec<Option<CompressedPTransaction<AnalyzedTransaction>>> = vec![None; block_size];
        let mut shard_idxs_by_txn: Vec<usize> = vec![0; block_size];
        let mut global_idxs: Vec<Vec<u32>> = vec![vec![]; num_shards];
        for (shard_idx, txns) in partition_result.into_iter().enumerate() {
            for (txn_idx, compressed_t) in txns {
                shard_idxs_by_txn[txn_idx] = shard_idx;
                global_idxs[shard_idx].push(txn_idx as u32);
                compressed_txns_by_global_idx[txn_idx] = Some(compressed_t);
            }
        }

        let mut dependency_sets: Vec<HashMap<u32, Vec<StateKey>>> = vec![HashMap::new(); block_size];
        let mut follower_sets: Vec<HashSet<u32>> = vec![HashSet::new(); block_size];
        let mut owners_by_key: HashMap<CompressedKey, u32> = HashMap::new();
        let mut sharded_txns: Vec<Vec<AnalyzedTransaction>> = vec![vec![]; num_shards];
        for (txn_idx, compressed_t) in compressed_txns_by_global_idx.into_iter().enumerate() {
            let t = Rc::try_unwrap(compressed_t.unwrap()).unwrap();
            let CompressedPTransactionInner{ original, read_set, write_set } = t;
            let analyzed_txn = *original;
            sharded_txns[shard_idxs_by_txn[txn_idx]].push(analyzed_txn);
            for key in read_set.iter().chain(write_set.iter()) {
                if let Some(src_txn_idx) = owners_by_key.get(key) {
                    dependency_sets[txn_idx].entry(*src_txn_idx).or_insert_with(Vec::new).push(compressor.uncompressed_key(*key as usize).clone());
                    follower_sets[*src_txn_idx as usize].insert(txn_idx as u32);
                }
            }
            for key in write_set.iter() {
                owners_by_key.insert(*key, txn_idx as u32);
            }
        }

        // for txn_idx in 0..block_size {
        //     println!("tid={}, deps={:?}, followers={:?}", txn_idx, dependency_sets[txn_idx], follower_sets[txn_idx]);
        // }

        PartitionedTransactions {
            block_id,
            sharded_txns,
            global_idxs,
            shard_idxs_by_txn,
            dependency_sets,
            follower_sets: follower_sets.into_iter().map(|txns|txns.into_iter().collect()).collect(),
        }
    }
}
