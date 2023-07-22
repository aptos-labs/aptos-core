// Copyright © Aptos Foundation

use crate::{analyze_block, BlockPartitioner, scheduling};
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, SubBlock, SubBlocksForShard, TransactionWithDependencies,
    },
    state_store::state_key::StateKey,
    transaction::{analyzed_transaction::AnalyzedTransaction, Transaction},
};
use itertools::Itertools;
use move_core_types::account_address::AccountAddress;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;
use rayon::{ThreadPool, ThreadPoolBuilder};
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator};
use aptos_metrics_core::{HistogramVec, register_histogram_vec, exponential_buckets};
use aptos_types::block_executor::partitioner::ShardedTxnIndex;
use crate::union_find::UnionFind;
use rayon::iter::ParallelIterator;

type Sender = Option<AccountAddress>;

pub struct SimplePartitioner {
    thread_pool: ThreadPool,
}
impl SimplePartitioner {
    pub fn new(concurrency_level: usize) -> Self {
        SimplePartitioner {
            thread_pool: ThreadPoolBuilder::new().num_threads(concurrency_level).build().unwrap()
        }
    }
    /// If `maybe_load_imbalance_tolerance` is none,
    /// it is guaranteed that the returned txn groups do not have cross-group conflicts.
    ///
    /// Also return `num_keys` to help optimize later stages.
    pub fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> (Vec<Vec<AnalyzedTransaction>>, usize) {
        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["group_by_sender"]).start_timer();
        let num_txns = txns.len();
        let mut txns_by_sender_id: Vec<Vec<AnalyzedTransaction>> = Vec::new();
        let mut key_ids_by_sender_id: Vec<DashSet<usize>> = Vec::new();
        let mut sender_ids_by_sender: HashMap<Sender, usize> = HashMap::new();
        for (txn_id, mut txn) in txns.into_iter().enumerate() {
            let sender = txn.sender();
            let sender_id = *sender_ids_by_sender.entry(sender).or_insert_with(||{
                let ret = txns_by_sender_id.len();
                txns_by_sender_id.push(vec![]);
                key_ids_by_sender_id.push(DashSet::new());
                ret
            });
            txn.maybe_txn_id_in_partition_session = Some(txn_id);
            txn.maybe_sender_id_in_partition_session = Some(sender_id);
            txns_by_sender_id[sender_id].push(txn);
        }
        let num_senders = txns_by_sender_id.len();
        let duration = timer.stop_and_record();
        // println!("group_by_sender={duration:?}");

        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["count_storage_locations"]).start_timer();
        let mut num_keys = AtomicUsize::new(0);
        let mut key_ids_by_key: DashMap<StateKey, usize> = DashMap::with_shard_amount(64);
        self.thread_pool.install(||{
            txns_by_sender_id.par_iter_mut().for_each(|txns|{
                for txn in txns {
                    for storage_location in txn.write_hints.iter_mut().chain(txn.read_hints.iter_mut()) {
                        let key = storage_location.maybe_state_key().unwrap().clone();
                        let key_id = *key_ids_by_key.entry(key).or_insert_with(||{
                            num_keys.fetch_add(1, Ordering::SeqCst)
                        });
                        storage_location.maybe_id_in_partition_session = Some(key_id);
                        key_ids_by_sender_id.get(txn.maybe_sender_id_in_partition_session.unwrap()).unwrap().insert(key_id);
                    }
                }
            });
        });
        let duration = timer.stop_and_record();
        // println!("count_storage_locations={duration:?}");
        /*
        Now txns_by_sender becomes:
        {
            Alice: [T_A3(K0, K1), T_A4(K0, K1)],
            Bob: [T_B0(K2), T_B1(K3, K99), T_B2(K2, K99), T_B3(K2, K3)],
            Carl: [T_C98(K2), T_C99(K3, K4, K5)],
        }
        */
        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["union_find"]).start_timer();
        // The union-find approach.
        let mut uf = UnionFind::new(num_senders + num_keys.load(Ordering::SeqCst));
        for (sender_id, key_ids) in key_ids_by_sender_id.iter().enumerate() {
            for key_id in key_ids.iter() {
                let key_id_in_uf = num_senders + *key_id.key();
                uf.union(key_id_in_uf, sender_id);
            }
        }

        let mut sender_groups_by_set_id: HashMap<usize, Vec<usize>> = HashMap::new();
        for sender_id in 0..num_senders {
            let set_id = uf.find(sender_id);
            sender_groups_by_set_id.entry(set_id).or_insert_with(Vec::new).push(sender_id);
        }
        let duration = timer.stop_and_record();
        // println!("simple_par/union_find={duration:?}");

        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["cap_group_size"]).start_timer();
        // If a sender group is too large,
        // break it into multiple sub-groups (and accept the fact that we will have cross-group conflicts),
        // each being small enough.

        let pct_imba_tolerance = std::env::var("SIMPLE_PARTITIONER__PCT_IMBA_TOLERANCE")
            .ok().map_or(None, |s|s.parse::<usize>().ok());
        let sub_group_size_limit = pct_imba_tolerance.map_or(u64::MAX, |pct| (num_txns * pct / (100 * num_executor_shards)) as u64);

        let capped_sender_groups: Vec<SenderGroup> = sender_groups_by_set_id.into_iter().flat_map(|(_set_id, sender_ids)|{
            let mut sub_groups: Vec<SenderGroup> = Vec::new();
            for sender_id in sender_ids {
                let num_txns_from_cur_sender = txns_by_sender_id[sender_id].len();
                if sub_groups.len() == 0 || sub_groups.last().unwrap().total_load + num_txns_from_cur_sender as u64 >= sub_group_size_limit {
                    sub_groups.push(SenderGroup::default());
                }
                sub_groups.last_mut().unwrap().add(sender_id, num_txns_from_cur_sender as u64);
            }
            sub_groups
        }).collect();

        /*
        Now capped_sender_groups becomes:
        [
            [A,X,Y],
            [B,C,D,E,F,G,H,I,J,K,L,M,N],
            [P,Q],
            [U,V],
            [R,S],
            [T],
            [Z],
        ]
        */

        let mut group_ids_by_sender_id: Vec<usize> = vec![usize::MAX; num_senders];
        for (gid, sender_group) in capped_sender_groups.iter().enumerate() {
            for sender_id in sender_group.sender_ids.iter() {
                group_ids_by_sender_id[*sender_id] = gid;
            }
        }
        let duration = timer.stop_and_record();
        //println!("simple_par/cap_group_size={}", duration);

        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["schedule"]).start_timer();
        let loads_by_sub_group: Vec<u64> = capped_sender_groups.iter().map(|g| g.total_load).collect();
        let (_, shard_ids_by_group_id) = scheduling::assign_tasks_to_workers(&loads_by_sub_group, num_executor_shards);
        timer.stop_and_record();

        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["build_return_object"]).start_timer();
        let mut txns_by_shard_id: Vec<Vec<AnalyzedTransaction>> = vec![vec![]; num_executor_shards];

        for (sender_id, txns) in txns_by_sender_id.into_iter().enumerate() {
            let group_id = group_ids_by_sender_id[sender_id];
            let shard_id = shard_ids_by_group_id[group_id];
            txns_by_shard_id.get_mut(shard_id).unwrap().extend(txns);
        }

        let duration = timer.stop_and_record();
        // println!("build_return_object={}", duration);
        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["drop"]).start_timer();
        drop(capped_sender_groups);
        drop(loads_by_sub_group);
        drop(group_ids_by_sender_id);
        drop(key_ids_by_sender_id);
        drop(key_ids_by_key);
        drop(sender_ids_by_sender);
        let duration = timer.stop_and_record();
        (txns_by_shard_id, num_keys.load(Ordering::SeqCst))
    }
}

impl BlockPartitioner for SimplePartitioner {
    fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> Vec<SubBlocksForShard<Transaction>> {
        let (txns_by_shard_id, num_keys) = self.partition(txns, num_executor_shards);
        let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["add_deps"]).start_timer();
        let mut ret: Vec<SubBlocksForShard<Transaction>> = Vec::with_capacity(num_executor_shards);
        let mut global_txn_counter: usize = 0;
        let mut global_owners_of_key: HashMap<StateKey, ShardedTxnIndex> = HashMap::new();
        for (shard_id, txns) in txns_by_shard_id.into_iter().enumerate() {
            let start_index_for_cur_sub_block = global_txn_counter;
            let mut twds_for_cur_sub_block: Vec<TransactionWithDependencies<Transaction>> = Vec::with_capacity(txns.len());
            let mut local_owners_of_key: HashMap<StateKey, ShardedTxnIndex> = HashMap::new();
            for txn in txns {
                let cur_sharded_txn_idx = ShardedTxnIndex {
                    txn_index: global_txn_counter,
                    shard_id,
                    round_id: 0,
                };
                let mut cur_txn_csd = CrossShardDependencies::default();
                for loc in txn.read_hints() {
                    let key = loc.clone().into_state_key();
                    match global_owners_of_key.get(&key) {
                        Some(owner) => {
                            ret.get_mut(owner.shard_id).unwrap()
                                .get_sub_block_mut(owner.round_id).unwrap()
                                .add_dependent_edge(owner.txn_index, cur_sharded_txn_idx, vec![loc.clone()]);
                            cur_txn_csd.add_required_edge(*owner, loc.clone());
                        },
                        None => {},
                    }
                }

                for loc in txn.write_hints() {
                    let key = loc.clone().into_state_key();
                    local_owners_of_key.insert(key.clone(), cur_sharded_txn_idx);
                    match global_owners_of_key.get(&key) {
                        Some(owner) => {
                            ret.get_mut(owner.shard_id).unwrap()
                                .get_sub_block_mut(owner.round_id).unwrap()
                                .add_dependent_edge(owner.txn_index, cur_sharded_txn_idx, vec![loc.clone()]);
                            cur_txn_csd.add_required_edge(*owner, loc.clone());
                        },
                        None => {},
                    }
                }

                let twd = TransactionWithDependencies::new(txn.into(), cur_txn_csd);
                twds_for_cur_sub_block.push(twd);
                global_txn_counter += 1;
            }

            let cur_sub_block = SubBlock::new(start_index_for_cur_sub_block, twds_for_cur_sub_block);
            ret.push(SubBlocksForShard::new(shard_id, vec![cur_sub_block]));

            for (key, owner) in local_owners_of_key {
                global_owners_of_key.insert(key, owner);
            }
        }
        ret
    }
}

pub static SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "simple_partitioner_misc_timers_seconds",
        // metric description
        "The time spent in seconds of miscellaneous phases of SimplePartitioner.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
        .unwrap()
});

struct SenderGroup {
    sender_ids: Vec<usize>,
    total_load: u64,
}

impl SenderGroup {
    fn add(&mut self, sender_id: usize, load: u64) {
        self.sender_ids.push(sender_id);
        self.total_load += load;
    }
}

impl Default for SenderGroup {
    fn default() -> Self {
        Self {
            sender_ids: vec![],
            total_load: 0,
        }
    }
}
