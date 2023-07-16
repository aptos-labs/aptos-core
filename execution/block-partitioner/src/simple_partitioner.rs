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
    ) -> Vec<Vec<Transaction>> {
        match std::env::var("SIMPLE_PARTITIONER__MERGE_WITH_UNION_FIND") {
            Ok(v) if v.as_str() == "1" => self.partition_uf(txns, num_executor_shards, maybe_load_imbalance_tolerance),
            _ => self.partition_bfs(txns, num_executor_shards, maybe_load_imbalance_tolerance),
        }
    }

    pub fn partition_uf(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
        maybe_load_imbalance_tolerance: Option<f32>
    ) -> Vec<Vec<Transaction>> {
        let num_txns = txns.len();

        let mut senders: Vec<Sender> = Vec::new();
        let mut num_keys: usize = 0;
        let mut sender_ids_by_sender: HashMap<Sender, usize> = HashMap::new();
        let mut key_ids_by_key: HashMap<StateKey, usize> = HashMap::new();
        let mut key_ids_by_sender_id: Vec<HashSet<usize>> = Vec::new();
        let mut txns_by_sender_id: Vec<Vec<Transaction>> = Vec::new();
        {
            let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["preprocess"]).start_timer();
            for (_txn_id, txn) in txns.into_iter().enumerate() {
                let sender = txn.sender();
                let sender_id = *sender_ids_by_sender.entry(sender.clone()).or_insert_with(||{
                    let ret = senders.len();
                    senders.push(sender);
                    txns_by_sender_id.push(Vec::new());
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
                txns_by_sender_id[sender_id].push(txn.into_txn());
            }
            println!("preprocess_time={}", timer.stop_and_record());
        }

        /*
        Now txns_by_sender becomes:
        {
            Alice: [T_A3(K0, K1), T_A4(K0, K1)],
            Bob: [T_B0(K2), T_B1(K3, K99), T_B2(K2, K99), T_B3(K2, K3)],
            Carl: [T_C98(K2), T_C99(K3, K4, K5)],
        }
        */
        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["union_find"]).start_timer();
        let mut num_groups: usize = 0;
        let mut group_ids_by_sender_id: HashMap<usize, usize> = HashMap::new();
        // The union-find approach.
        let mut uf = UnionFind::new(senders.len() + num_keys);
        for (sender_id, key_ids) in key_ids_by_sender_id.iter().enumerate() {
            for &key_id in key_ids.iter() {
                let key_id_in_uf = senders.len() + key_id;
                uf.union(key_id_in_uf, sender_id);
            }
        }

        let mut group_ids_by_set_id: HashMap<usize, usize> = HashMap::new();
        for sender_id in 0..senders.len() {
            let set_id = uf.find(sender_id);
            let group_id = group_ids_by_set_id.entry(set_id).or_insert_with(||{
                let ret = num_groups;
                num_groups += 1;
                ret
            });
            group_ids_by_sender_id.insert(sender_id, *group_id);
        }
        timer.stop_and_record();

        /*
        Now group_ids_by_sender becomes:
        {
            A: 0,
            B: 1,
            C: 1,
            ...
            P: 2,
            Q: 2,
            ....
            X: 0,
            Y: 0,
        }
        */

        // If a sender group is too large,
        // break it into multiple sub-groups (and accept the fact that we will have cross-group conflicts),
        // each being small enough.
        let sub_group_size_limit = maybe_load_imbalance_tolerance.map_or(usize::MAX, |k| {
            let x: usize = ((num_txns as f32) * k / (num_executor_shards as f32)) as usize;
            x
        });
        let mut cur_sug_group_ids_by_group_id: Vec<usize> = (0..num_groups).collect();
        let mut sub_groups: Vec<Vec<usize>> = vec![vec![]; num_groups];
        let mut loads_by_sub_group: Vec<usize> = vec![0; num_groups];
        {
            let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["cap_group_size"]).start_timer();
            for (sender_id, txns) in txns_by_sender_id.iter().enumerate() {
                let group_id = *group_ids_by_sender_id.get(&sender_id).unwrap();
                let sub_group_id = cur_sug_group_ids_by_group_id[group_id];
                if loads_by_sub_group[sub_group_id] == 0 || loads_by_sub_group[sub_group_id] + txns.len() < sub_group_size_limit {
                    sub_groups.get_mut(sub_group_id).unwrap().push(sender_id);
                    loads_by_sub_group[sub_group_id] += txns.len();
                } else {
                    let new_sub_group_id = sub_groups.len();
                    cur_sug_group_ids_by_group_id[group_id] = new_sub_group_id;
                    sub_groups.push(vec![sender_id]);
                    loads_by_sub_group.push(txns.len());
                }
            }
        }
        /*
        Now sub_groups becomes:
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
        let (_, shard_ids_by_sub_group_id) = scheduling::assign_tasks_to_workers(loads_by_sub_group, num_executor_shards);

        let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["build_return_object"]).start_timer();
        let mut txns_by_shard_id: Vec<Vec<Transaction>> = vec![vec![]; num_executor_shards];
        for (sender_id, txns) in txns_by_sender_id.into_iter().enumerate() {
            let group_id = *group_ids_by_sender_id.get(&sender_id).unwrap();
            let sub_group_id = cur_sug_group_ids_by_group_id[group_id];
            let shard_id = shard_ids_by_sub_group_id[sub_group_id];
            txns_by_shard_id.get_mut(shard_id).unwrap().extend(txns);
        }
        txns_by_shard_id
    }

    pub fn partition_bfs(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
        maybe_load_imbalance_tolerance: Option<f32>
    ) -> Vec<Vec<Transaction>> {
        let num_txns = txns.len();

        let mut senders: Vec<Sender> = Vec::new();
        let mut keys: Vec<StateKey> = Vec::new();
        let mut sender_ids_by_sender: HashMap<Sender, usize> = HashMap::new();
        let mut key_ids_by_key: HashMap<StateKey, usize> = HashMap::new();
        let mut key_ids_by_sender_id: Vec<HashSet<usize>> = Vec::new();
        let mut txns_by_sender_id: Vec<Vec<Transaction>> = Vec::new();
        let mut sender_ids_by_key_id: Vec<HashSet<usize>> = Vec::new();
        {
            let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["preprocess"]).start_timer();
            for (txn_id, txn) in txns.into_iter().enumerate() {
                let sender = txn.sender();
                let sender_id = sender_ids_by_sender.entry(sender.clone()).or_insert_with(||{
                    let ret = senders.len();
                    senders.push(sender);
                    key_ids_by_sender_id.push(HashSet::new());
                    txns_by_sender_id.push(Vec::new());
                    ret
                });
                for storage_location in txn.write_hints().iter().chain(txn.read_hints().iter()) {
                    let key = storage_location.clone().into_state_key();
                    let key_id = key_ids_by_key.entry(key.clone()).or_insert_with(||{
                        let ret = keys.len();
                        keys.push(key.clone());
                        sender_ids_by_key_id.push(HashSet::new());
                        ret
                    });
                    sender_ids_by_key_id[*key_id].insert(*sender_id);
                    key_ids_by_sender_id[*sender_id].insert(*key_id);
                }
                txns_by_sender_id[*sender_id].push(txn.into_txn());
            }
        }

        /*
        Now txns_by_sender becomes:
        {
            Alice: [T_A3(K0, K1), T_A4(K0, K1)],
            Bob: [T_B0(K2), T_B1(K3, K99), T_B2(K2, K99), T_B3(K2, K3)],
            Carl: [T_C98(K2), T_C99(K3, K4, K5)],
        }
        */
        let mut num_groups: usize = 0;
        let mut group_ids_by_sender_id: HashMap<usize, usize> = HashMap::new();
        let timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["group_with_bfs"]).start_timer();
        for sender_id in 0..senders.len() {
            if !group_ids_by_sender_id.contains_key(&sender_id) {
                // BFS initialization.
                let mut sender_ids_to_explore: VecDeque<usize> = VecDeque::new();
                sender_ids_to_explore.push_back(sender_id);
                group_ids_by_sender_id.insert(sender_id, num_groups);

                while let Some(cur_sender_id) = sender_ids_to_explore.pop_front() {
                    for &key_id in key_ids_by_sender_id[cur_sender_id].iter() {
                        for &nxt_sender_id in sender_ids_by_key_id[key_id].iter() {
                            if !group_ids_by_sender_id.contains_key(&nxt_sender_id) {
                                sender_ids_to_explore.push_back(nxt_sender_id);
                                group_ids_by_sender_id.insert(nxt_sender_id, num_groups);
                            }
                        }
                    }
                }

                num_groups += 1;
            }
        }
        timer.stop_and_record();
        /*
        Now group_ids_by_sender becomes:
        {
            A: 0,
            B: 1,
            C: 1,
            ...
            P: 2,
            Q: 2,
            ....
            X: 0,
            Y: 0,
        }
        */

        // If a sender group is too large,
        // break it into multiple sub-groups (and accept the fact that we will have cross-group conflicts),
        // each being small enough.
        let sub_group_size_limit = maybe_load_imbalance_tolerance.map_or(usize::MAX, |k| {
            let x: usize = ((num_txns as f32) * k / (num_executor_shards as f32)) as usize;
            x
        });
        let mut cur_sug_group_ids_by_group_id: Vec<usize> = (0..num_groups).collect();
        let mut sub_groups: Vec<Vec<usize>> = vec![vec![]; num_groups];
        let mut loads_by_sub_group: Vec<usize> = vec![0; num_groups];
        {
            let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["cap_group_size"]).start_timer();
            for (sender_id, txns) in txns_by_sender_id.iter().enumerate() {
                let group_id = *group_ids_by_sender_id.get(&sender_id).unwrap();
                let sub_group_id = cur_sug_group_ids_by_group_id[group_id];
                if loads_by_sub_group[sub_group_id] == 0 || loads_by_sub_group[sub_group_id] + txns.len() < sub_group_size_limit {
                    sub_groups.get_mut(sub_group_id).unwrap().push(sender_id);
                    loads_by_sub_group[sub_group_id] += txns.len();
                } else {
                    let new_sub_group_id = sub_groups.len();
                    cur_sug_group_ids_by_group_id[group_id] = new_sub_group_id;
                    sub_groups.push(vec![sender_id]);
                    loads_by_sub_group.push(txns.len());
                }
            }
        }
        /*
        Now sub_groups becomes:
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
        let (_, shard_ids_by_sub_group_id) = scheduling::assign_tasks_to_workers(loads_by_sub_group, num_executor_shards);

        let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["build_return_object"]).start_timer();
        let mut txns_by_shard_id: Vec<Vec<Transaction>> = vec![vec![]; num_executor_shards];
        for (sender_id, txns) in txns_by_sender_id.into_iter().enumerate() {
            let group_id = *group_ids_by_sender_id.get(&sender_id).unwrap();
            let sub_group_id = cur_sug_group_ids_by_group_id[group_id];
            let shard_id = shard_ids_by_sub_group_id[sub_group_id];
            txns_by_shard_id.get_mut(shard_id).unwrap().extend(txns);
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
        let txns_by_shard_id = self.partition(txns, num_executor_shards, None);
        let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["convert_to_SBFSs"]).start_timer();
        let mut ret = Vec::with_capacity(num_executor_shards);
        let mut txn_counter: usize = 0;
        for (shard_id, txns) in txns_by_shard_id.into_iter().enumerate() {
            let twds: Vec<TransactionWithDependencies<Transaction>> = txns
                .into_iter()
                .map(|txn| TransactionWithDependencies::new(txn, CrossShardDependencies::default()))
                .collect();
            let aggregated_sub_block = SubBlock::new(txn_counter, twds);
            txn_counter += aggregated_sub_block.num_txns();
            let sub_block_list = SubBlocksForShard::new(shard_id, vec![aggregated_sub_block]);
            ret.push(sub_block_list);
        }
        let worker_loads: Vec<usize> = ret.iter().map(|sbl| sbl.num_txns()).collect();
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
