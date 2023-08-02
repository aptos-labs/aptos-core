// Copyright © Aptos Foundation

use std::cmp::min;
use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use once_cell::sync::Lazy;
use aptos_block_partitioner::BlockPartitioner;
use aptos_crypto::hash::CryptoHash;
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_metrics_core::{HistogramVec, register_histogram_vec};
use aptos_types::block_executor::partitioner::{CrossShardDependencies, ShardedTxnIndex, SubBlock, SubBlocksForShard, TransactionWithDependencies};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use crate::batch_orderer::{SequentialDynamicAriaOrderer, WindowManager};
use crate::block_orderer::{BatchedBlockOrdererWithWindow, BlockOrderer};
use crate::block_partitioner::OrderedRoundRobinPartitioner;
use crate::const_option::ConstSome;
use aptos_metrics_core::exponential_buckets;

pub struct V3Partitioner {
    num_rounds: usize,
    block_size: usize,
    block_orderer: BatchedBlockOrdererWithWindow<SequentialDynamicAriaOrderer<AnalyzedTransaction, ConstSome<WindowManager<StateKey>>>>,
}

impl V3Partitioner {
    pub fn new() -> Self {
        let block_size = std::env::var("APTOS_BLOCK_PARTITIONER_V3__BLOCK_SIZE").ok().map(|v|v.parse::<usize>().ok().unwrap_or(100000)).unwrap_or(100000);
        let num_rounds = std::env::var("APTOS_BLOCK_PARTITIONER_V3__NUM_ROUNDS").ok().map(|v|v.parse::<usize>().ok().unwrap_or(4)).unwrap_or(4);
        info!("Creating V3Partitioner with block_size={}, num_rounds={}", block_size, num_rounds);
        let min_ordered_transaction_before_execution = min(100, block_size);
        let block_orderer = BatchedBlockOrdererWithWindow::new(
            SequentialDynamicAriaOrderer::with_window(),
            min_ordered_transaction_before_execution * 5,
            1000,
        );

        Self {
            num_rounds,
            block_size,
            block_orderer,
        }
    }
}
impl BlockPartitioner for V3Partitioner {
    fn partition(&self, mut txns: Vec<AnalyzedTransaction>, num_shards: usize) -> Vec<SubBlocksForShard<AnalyzedTransaction>> {
        assert_eq!(self.block_size, txns.len());

        let timer = MISC_TIMERS_SECONDS.with_label_values(&["preprocess"]).start_timer();
        let mut loc_id_by_loc: HashMap<HashValue, usize> = HashMap::new();
        let num_keys = AtomicUsize::new(0);
        txns.iter_mut().enumerate().for_each(|(tid, t)|{
            for loc in t.write_hints.iter_mut().chain(t.read_hints.iter_mut()) {
                let loc_id = *loc_id_by_loc.entry(loc.state_key().hash()).or_insert_with(||{
                    num_keys.fetch_add(1, Ordering::SeqCst)
                });
                loc.maybe_loc_id = Some(loc_id);
            }
        });
        let num_keys = num_keys.load(Ordering::SeqCst);
        let duration = timer.stop_and_record();
        println!("preprocess={}", duration);

        let timer = MISC_TIMERS_SECONDS.with_label_values(&["main"]).start_timer();
        let mut txns_by_shard_id = vec![vec![]; num_shards];
        let mut next_shard_id_to_receive: usize = 0;
        self.block_orderer.order_transactions(txns, |txns| -> Result<(), io::Error> {
            for txn in txns {
                txns_by_shard_id[next_shard_id_to_receive].push(txn);
                next_shard_id_to_receive += 1;
                next_shard_id_to_receive %= num_shards;
            }
            Ok(())
        }).unwrap();
        let duration = timer.stop_and_record();
        println!("main={}", duration);

        let timer = MISC_TIMERS_SECONDS.with_label_values(&["convert_to_matrix"]).start_timer();
        let height = txns_by_shard_id[0].len();
        let num_big_rounds = height % self.num_rounds;
        let small_round_size = height / self.num_rounds;
        let mut matrix = vec![vec![vec![]; num_shards]; self.num_rounds];
        let mut start: usize = 0;
        for round_id in 0..self.num_rounds {
            let round_size = if round_id < num_big_rounds {
                small_round_size + 1
            } else {
                small_round_size
            };
            let end = start + round_size;
            for shard_id in 0..num_shards {
                matrix[round_id][shard_id] = txns_by_shard_id[shard_id][start..end].to_vec();
            }
            start = end;
        }
        let duration = timer.stop_and_record();
        println!("convert_to_matrix={}", duration);

        let timer = MISC_TIMERS_SECONDS.with_label_values(&["add_edges"]).start_timer();
        let ret = add_edges(matrix, num_keys);
        let duration = timer.stop_and_record();
        println!("add_edges={}", duration);

        ret
    }
}

fn add_edges(
    matrix: Vec<Vec<Vec<AnalyzedTransaction>>>,
    num_keys: usize,
) -> Vec<SubBlocksForShard<AnalyzedTransaction>> {
    let timer = MISC_TIMERS_SECONDS.with_label_values(&["add_edges__main"]).start_timer();
    let num_shards = matrix[0].len();
    let mut ret: Vec<SubBlocksForShard<AnalyzedTransaction>> = (0..num_shards).map(|shard_id| SubBlocksForShard { shard_id, sub_blocks: vec![] }).collect();
    let mut global_txn_counter: usize = 0;
    let mut global_owners_by_loc_id: Vec<Option<ShardedTxnIndex>> = vec![None; num_keys];// HashMap<StateKey, ShardedDTxnIndex>;
    for (round_id, row) in matrix.into_iter().enumerate() {
        for (shard_id, txns) in row.into_iter().enumerate() {
            let start_index_for_cur_sub_block = global_txn_counter;
            let mut twds_for_cur_sub_block: Vec<TransactionWithDependencies<AnalyzedTransaction>> = Vec::with_capacity(txns.len());
            let mut local_owners_by_loc_id: HashMap<usize, ShardedTxnIndex> = HashMap::new();
            for txn in txns {
                let cur_sharded_txn_idx = ShardedTxnIndex {
                    txn_index: global_txn_counter,
                    shard_id,
                    round_id,
                };
                let mut cur_txn_csd = CrossShardDependencies::default();
                for loc in txn.read_hints().iter() {
                    let loc_id = *loc.maybe_loc_id.as_ref().unwrap();
                    match &global_owners_by_loc_id[loc_id] {
                        Some(owner) => {
                            let timer = MISC_TIMERS_SECONDS.with_label_values(&["loc_clone"]).start_timer();
                            let loc_clone_1 = loc.clone();
                            let loc_clone_2 = loc.clone();
                            timer.stop_and_record();

                            let timer = MISC_TIMERS_SECONDS.with_label_values(&["add_dependent"]).start_timer();
                            ret.get_mut(owner.shard_id).unwrap()
                                .get_sub_block_mut(owner.round_id).unwrap()
                                .add_dependent_edge(owner.txn_index, cur_sharded_txn_idx, vec![loc_clone_1]);
                            timer.stop_and_record();

                            let timer = MISC_TIMERS_SECONDS.with_label_values(&["add_required"]).start_timer();
                            cur_txn_csd.add_required_edge(*owner, loc_clone_2);
                            timer.stop_and_record();
                        },
                        None => {},
                    }
                }

                for loc in txn.write_hints().iter() {
                    let loc_id = *loc.maybe_loc_id.as_ref().unwrap();
                    let timer = MISC_TIMERS_SECONDS.with_label_values(&["local_owner_update"]).start_timer();
                    local_owners_by_loc_id.insert(loc_id, cur_sharded_txn_idx);
                    timer.stop_and_record();
                    match &global_owners_by_loc_id[loc_id] {
                        Some(owner) => {
                            let timer = MISC_TIMERS_SECONDS.with_label_values(&["loc_clone"]).start_timer();
                            let loc_clone_1 = loc.clone();
                            let loc_clone_2 = loc.clone();
                            timer.stop_and_record();

                            let timer = MISC_TIMERS_SECONDS.with_label_values(&["add_dependent"]).start_timer();
                            let x = ret.get_mut(owner.shard_id).unwrap()
                                .get_sub_block_mut(owner.round_id).unwrap();
                            x.add_dependent_edge(owner.txn_index, cur_sharded_txn_idx, vec![loc_clone_1]);
                            timer.stop_and_record();

                            let timer = MISC_TIMERS_SECONDS.with_label_values(&["add_required"]).start_timer();
                            cur_txn_csd.add_required_edge(*owner, loc_clone_2);
                            timer.stop_and_record();
                        },
                        None => {},
                    }
                }
                let twd = TransactionWithDependencies::new(txn, cur_txn_csd);
                twds_for_cur_sub_block.push(twd);
                global_txn_counter += 1;
            }

            let cur_sub_block = SubBlock::new(start_index_for_cur_sub_block, twds_for_cur_sub_block);
            ret.get_mut(shard_id).unwrap().add_sub_block(cur_sub_block);

            let timer = MISC_TIMERS_SECONDS.with_label_values(&["global_owner_update"]).start_timer();
            for (key, owner) in local_owners_by_loc_id {
                global_owners_by_loc_id[key] = Some(owner);
            }
            timer.stop_and_record();
        }
    }
    let duration = timer.stop_and_record();
    let timer = MISC_TIMERS_SECONDS.with_label_values(&["drop"]).start_timer();
    drop(global_owners_by_loc_id);
    let duration = timer.stop_and_record();
    ret
}


pub static MISC_TIMERS_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_block_partitioner_v3_misc_timers_seconds",
        // metric description
        "The time spent in seconds of miscellaneous phases of block partitioner v3.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
        .unwrap()
});
