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
use std::time::Instant;
use once_cell::sync::Lazy;
use aptos_metrics_core::{HistogramVec, register_histogram_vec, exponential_buckets};
use aptos_types::block_executor::partitioner::ShardedTxnIndex;
use crate::union_find::UnionFind;

type Sender = Option<AccountAddress>;

pub struct SimplePartitioner {}
impl SimplePartitioner {
    /// If `maybe_load_imbalance_tolerance` is none,
    /// it is guaranteed that the returned txn groups do not have cross-group conflicts.
    pub fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
        maybe_load_imbalance_tolerance: Option<f32>
    ) -> Vec<Vec<AnalyzedTransaction>> {
        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["preprocess"]).start_timer();
        let num_txns = txns.len();
        let mut senders: Vec<Sender> = Vec::new();
        let mut num_keys: usize = 0;
        let mut key_ids_by_sender_id: Vec<HashSet<usize>> = Vec::new();
        let mut txns_by_sender_id: HashMap<usize, Vec<AnalyzedTransaction>> = HashMap::new();
        let mut key_ids_by_key: HashMap<StateKey, usize> = HashMap::new();
        let mut sender_ids_by_sender: HashMap<Sender, usize> = HashMap::new();
        for (_txn_id, txn) in txns.into_iter().enumerate() {
            let sender = txn.sender();
            let sender_id = *sender_ids_by_sender.entry(sender.clone()).or_insert_with(||{
                let ret = senders.len();
                senders.push(sender);
                key_ids_by_sender_id.push(HashSet::new());
                ret
            });
            for storage_location in txn.write_hints().iter().chain(txn.read_hints().iter()) {
                let key = storage_location.clone().into_state_key();
                let key_id = *key_ids_by_key.entry(key.clone()).or_insert_with(||{
                    let ret = num_keys;
                    num_keys += 1;
                    ret
                });
                key_ids_by_sender_id[sender_id].insert(key_id);
            }
            txns_by_sender_id.entry(sender_id).or_insert_with(Vec::new).push(txn);
        }
        timer.stop_and_record();

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
        let mut uf = UnionFind::new(senders.len() + num_keys);
        for (sender_id, key_ids) in key_ids_by_sender_id.iter().enumerate() {
            for &key_id in key_ids.iter() {
                let key_id_in_uf = senders.len() + key_id;
                uf.union(key_id_in_uf, sender_id);
            }
        }

        let mut sender_groups_by_set_id: HashMap<usize, Vec<usize>> = HashMap::new();
        for sender_id in 0..senders.len() {
            let set_id = uf.find(sender_id);
            sender_groups_by_set_id.entry(set_id).or_insert_with(Vec::new).push(sender_id);
        }
        timer.stop_and_record();

        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["cap_group_size"]).start_timer();
        // If a sender group is too large,
        // break it into multiple sub-groups (and accept the fact that we will have cross-group conflicts),
        // each being small enough.
        let sub_group_size_limit = maybe_load_imbalance_tolerance.map_or(u64::MAX, |k| {
            ((num_txns as f32) * k / (num_executor_shards as f32)) as u64
        });

        let capped_sender_groups: Vec<SenderGroup> = sender_groups_by_set_id.into_iter().flat_map(|(_set_id, sender_ids)|{
            let mut sub_groups: Vec<SenderGroup> = Vec::new();
            for sender_id in sender_ids {
                let num_txns_from_cur_sender = txns_by_sender_id.get(&sender_id).unwrap().len();
                if sub_groups.len() == 0 || sub_groups.last().unwrap().total_load + num_txns_from_cur_sender as u64 >= sub_group_size_limit {
                    sub_groups.push(SenderGroup::default());
                }
                sub_groups.last_mut().unwrap().add(sender_id, num_txns_from_cur_sender as u64);
            }
            sub_groups
        }).collect();
        timer.stop_and_record();

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

        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["schedule"]).start_timer();
        let loads_by_sub_group: Vec<u64> = capped_sender_groups.iter().map(|g| g.total_load).collect();
        let (_, shard_ids_by_group_id) = scheduling::assign_tasks_to_workers(&loads_by_sub_group, num_executor_shards);
        timer.stop_and_record();

        let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["build_return_object"]).start_timer();
        let mut txns_by_shard_id: Vec<Vec<AnalyzedTransaction>> = vec![vec![]; num_executor_shards];
        for (gid, group) in capped_sender_groups.into_iter().enumerate() {
            let shard_id = shard_ids_by_group_id[gid];
            let txns_for_shard = txns_by_shard_id.get_mut(shard_id).unwrap();
            for sender_id in group.sender_ids {
                let txns = txns_by_sender_id.remove(&sender_id).unwrap();
                txns_for_shard.extend(txns);
            }
        }
        txns_by_shard_id
    }
}

impl BlockPartitioner for SimplePartitioner {
    fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> Vec<SubBlocksForShard<Transaction>> {
        let txns_by_shard_id = self.partition(txns, num_executor_shards, Some(2.0));
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
