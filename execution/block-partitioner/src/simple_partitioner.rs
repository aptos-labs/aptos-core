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
        let num_txns = txns.len();

        // Sender-to-keyset and keyset-to-sender lookup table.
        let mut senders_by_key: HashMap<StateKey, HashSet<Sender>> = HashMap::new();
        let mut keys_by_sender: HashMap<Sender, HashSet<StateKey>> = HashMap::new();

        // Sender-to-tidset look-up table.
        let mut txns_by_sender: HashMap<Sender, Vec<Transaction>> = HashMap::new();

        let mut num_keys: usize = 0;
        let mut key_ids_by_key: HashMap<StateKey, usize> = HashMap::new();
        let mut num_senders: usize = 0;
        let mut sender_ids_by_sender: HashMap<Sender, usize> = HashMap::new();
        {
            let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["preprocess"]).start_timer();
            for (_tid, txn) in txns.into_iter().enumerate() {
                let sender = txn.sender();
                let _sender_id = sender_ids_by_sender.entry(sender).or_insert_with(||{
                    let ret = num_senders;
                    num_senders += 1;
                    ret
                });
                for storage_location in txn.write_hints().iter().chain(txn.read_hints().iter()) {
                    let key = storage_location.clone().into_state_key();
                    let _key_id = key_ids_by_key.entry(key.clone()).or_insert_with(||{
                        let ret = num_keys;
                        num_keys += 1;
                        ret
                    });
                    senders_by_key
                        .entry(key.clone())
                        .or_insert_with(HashSet::new)
                        .insert(sender);
                    keys_by_sender
                        .entry(sender)
                        .or_insert_with(HashSet::new)
                        .insert(key);
                }
                txns_by_sender
                    .entry(txn.sender())
                    .or_insert_with(Vec::new)
                    .push(txn.into_txn());
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
        let mut group_ids_by_sender: HashMap<Sender, usize> = HashMap::new();
        match std::env::var("SIMPLE_PARTITIONER__MERGE_WITH_UNION_FIND") {
            Ok(v) if v.as_str() == "1" => {
                let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["union_find"]).start_timer();
                // The union-find approach.
                let mut uf = UnionFind::new(num_senders + num_keys);
                for (key, senders) in senders_by_key.iter() {
                    let key_id = *key_ids_by_key.get(key).unwrap();
                    let key_id_in_uf = num_senders + key_id;
                    for sender in senders.iter() {
                        let sender_id= *sender_ids_by_sender.get(sender).unwrap();
                        uf.union(key_id_in_uf, sender_id);
                    }
                }

                let mut group_ids_by_set_id: HashMap<usize, usize> = HashMap::new();
                for (sender, sender_id) in sender_ids_by_sender.iter() {
                    let set_id = uf.find(*sender_id);
                    let group_id = group_ids_by_set_id.entry(set_id).or_insert_with(||{
                        let ret = num_groups;
                        num_groups += 1;
                        ret
                    });
                    group_ids_by_sender.insert(sender.clone(), *group_id);
                }
            }
            _ => {
                let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["group_with_bfs"]).start_timer();
                for (sender, _tid_list) in txns_by_sender.iter() {
                    if !group_ids_by_sender.contains_key(sender) {
                        // BFS initialization.
                        let mut senders_to_explore: VecDeque<&Sender> = VecDeque::new();
                        senders_to_explore.push_back(sender);
                        group_ids_by_sender.insert(*sender, num_groups);

                        while let Some(cur_sender) = senders_to_explore.pop_front() {
                            for key in keys_by_sender.get(cur_sender).unwrap().iter() {
                                for nxt_sender in senders_by_key.get(key).unwrap().iter() {
                                    if !group_ids_by_sender.contains_key(nxt_sender) {
                                        senders_to_explore.push_back(nxt_sender);
                                        group_ids_by_sender.insert(*nxt_sender, num_groups);
                                    }
                                }
                            }
                        }

                        num_groups += 1;
                    }
                }
            }
        }
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
        let mut sub_groups: Vec<Vec<Sender>> = vec![vec![]; num_groups];
        let mut loads_by_sub_group: Vec<usize> = vec![0; num_groups];
        {
            let _timer = SIMPLE_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["cap_group_size"]).start_timer();
            for (sender, txns) in txns_by_sender.iter() {
                let group_id = *group_ids_by_sender.get(sender).unwrap();
                let sub_group_id = cur_sug_group_ids_by_group_id[group_id];
                if loads_by_sub_group[sub_group_id] == 0 || loads_by_sub_group[sub_group_id] + txns.len() < sub_group_size_limit {
                    sub_groups.get_mut(sub_group_id).unwrap().push(*sender);
                    loads_by_sub_group[sub_group_id] += txns.len();
                } else {
                    let new_sub_group_id = sub_groups.len();
                    cur_sug_group_ids_by_group_id[group_id] = new_sub_group_id;
                    sub_groups.push(vec![*sender]);
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
        for (sub_group_id, sub_group) in sub_groups.into_iter().enumerate() {
            let shard_id = *shard_ids_by_sub_group_id.get(sub_group_id).unwrap();
            for sender in sub_group {
                let txns = txns_by_sender.remove(&sender).unwrap();
                txns_by_shard_id.get_mut(shard_id).unwrap().extend(txns);
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
