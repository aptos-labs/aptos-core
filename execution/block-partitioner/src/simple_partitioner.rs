// Copyright © Aptos Foundation

use crate::{analyze_block, BlockPartitioner};
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

type Sender = Option<AccountAddress>;

pub struct SimplePartitioner {}

impl BlockPartitioner for SimplePartitioner {
    fn partition(
        &self,
        txns: Vec<Transaction>,
        num_executor_shards: usize,
    ) -> Vec<SubBlocksForShard<Transaction>> {
        let txns = analyze_block(txns);

        // Sender-to-keyset and keyset-to-sender lookup table.
        let mut senders_by_key: HashMap<StateKey, HashSet<Sender>> = HashMap::new();
        let mut keys_by_sender: HashMap<Sender, HashSet<StateKey>> = HashMap::new();
        for txn in txns.iter() {
            let sender = txn.sender();
            for write_hint in txn.write_hints() {
                let key = write_hint.clone().into_state_key();
                senders_by_key
                    .entry(key.clone())
                    .or_insert_with(HashSet::new)
                    .insert(sender);
                keys_by_sender
                    .entry(sender)
                    .or_insert_with(HashSet::new)
                    .insert(key);
            }
        }

        // Sender-to-tidset look-up table.
        let mut txns_by_sender: HashMap<Sender, Vec<Transaction>> = HashMap::new();
        for (_tid, txn) in txns.into_iter().enumerate() {
            txns_by_sender
                .entry(txn.sender())
                .or_insert_with(Vec::new)
                .push(txn.into_txn());
        }

        let mut num_groups: usize = 0;
        let mut group_ids_by_sender: HashMap<Sender, usize> = HashMap::new();
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

        let mut sender_groups: Vec<Vec<Sender>> = vec![vec![]; num_groups];
        for (sender, _) in txns_by_sender.iter() {
            let &group_id = group_ids_by_sender.get(sender).unwrap();
            sender_groups.get_mut(group_id).unwrap().push(*sender);
        }

        let group_sizes: Vec<usize> = sender_groups
            .iter()
            .map(|g| g.iter().map(|s| txns_by_sender.get(s).unwrap().len()).sum())
            .collect();
        info!("group_sizes={:?}", &group_sizes);
        let (_, shard_ids_by_gid) = assign_tasks_to_workers(group_sizes, num_executor_shards);

        let mut txns_by_shard_id: Vec<Vec<Transaction>> = vec![vec![]; num_executor_shards];
        for (sender, txns) in txns_by_sender.into_iter() {
            let group_id = *group_ids_by_sender.get(&sender).unwrap();
            let shard_id = *shard_ids_by_gid.get(group_id).unwrap();
            txns_by_shard_id.get_mut(shard_id).unwrap().extend(txns);
        }

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
        info!("worker_loads={:?}", worker_loads);
        ret
    }
}

fn assign_tasks_to_workers(mut tasks: Vec<usize>, num_workers: usize) -> (usize, Vec<usize>) {
    assert!(num_workers >= 1);
    tasks.sort_by(|a, b| b.cmp(a));
    let mut worker_prio_heap: BinaryHeap<(usize, usize)> =
        BinaryHeap::from((0..num_workers).map(|wid| (usize::MAX, wid)).collect_vec());
    let mut worker_ids_by_tid = Vec::with_capacity(tasks.len());
    for task in tasks.iter() {
        let (availability, worker_id) = worker_prio_heap.pop().unwrap();
        worker_ids_by_tid.push(worker_id);
        let new_availability = availability - task;
        worker_prio_heap.push((new_availability, worker_id));
    }
    let longest_pole = worker_prio_heap
        .into_iter()
        .map(|(a, _i)| usize::MAX - a)
        .max()
        .unwrap();
    (longest_pole, worker_ids_by_tid)
}

#[test]
fn test1() {
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 1);
    assert_eq!(15, actual);
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 2);
    assert_eq!(8, actual);
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 3);
    assert_eq!(5, actual);
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 4);
    assert_eq!(5, actual);
    let (actual, _) = assign_tasks_to_workers(vec![1, 2, 3, 4, 5], 5);
    assert_eq!(5, actual);
}
