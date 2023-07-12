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
use std::time::Instant;
use crate::scheduling::assign_tasks_to_workers;
use crate::union_find::UnionFind;

type Sender = Option<AccountAddress>;

pub struct SimpleUfPartitioner {}

impl BlockPartitioner for SimpleUfPartitioner {
    fn partition(
        &self,
        txns: Vec<Transaction>,
        num_executor_shards: usize,
    ) -> Vec<SubBlocksForShard<Transaction>> {
        let timer = Instant::now();
        let txns = analyze_block(txns);
        println!("analyze_time={:?}", timer.elapsed());

        // Sender-to-keyset and keyset-to-sender lookup table.
        let mut senders_by_key: HashMap<StateKey, HashSet<Sender>> = HashMap::new();
        let mut keys_by_sender: HashMap<Sender, HashSet<StateKey>> = HashMap::new();
        let mut num_senders: usize = 0;
        let mut sender_ids_by_sender: HashMap<Sender, usize> = HashMap::new();
        let mut num_keys: usize = 0;
        let mut key_ids_by_key: HashMap<StateKey, usize> = HashMap::new();

        // Sender-to-tidset look-up table.
        let mut txns_by_sender: HashMap<Sender, Vec<Transaction>> = HashMap::new();

        {
            let timer = Instant::now();
            for txn in txns.iter() {
                let sender = txn.sender();
                let sender_id = sender_ids_by_sender.entry(sender).or_insert_with(||{
                    let ret = num_senders;
                    num_senders += 1;
                    ret
                });
                for write_hint in txn.write_hints() {
                    let key = write_hint.clone().into_state_key();
                    let key_id = key_ids_by_key.entry(key.clone()).or_insert_with(||{
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
            }

            for (_tid, txn) in txns.into_iter().enumerate() {
                txns_by_sender
                    .entry(txn.sender())
                    .or_insert_with(Vec::new)
                    .push(txn.into_txn());
            }
            println!("preprocessing_time={:?}", timer.elapsed());
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
        {
            let timer = Instant::now();
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
            println!("uf_approach_time={:?}", timer.elapsed());
        }
        /*
        Now group_ids_by_sender becomes:
        {
            Alice: 0,
            Bob: 1,
            Carl: 1,
        }
        */

        let mut sender_groups: Vec<Vec<Sender>> = vec![vec![]; num_groups];
        {
            let timer = Instant::now();
            for (sender, _) in txns_by_sender.iter() {
                let &group_id = group_ids_by_sender.get(sender).unwrap();
                sender_groups.get_mut(group_id).unwrap().push(*sender);
            }
            println!("sender_groups_update_time={:?}", timer.elapsed());
        }
        /*
        Now sender_groups becomes:
        [
            [Alice],
            [Bob, Carl],
        ]
        */
        let timer = Instant::now();
        let group_sizes: Vec<usize> = sender_groups
            .iter()
            .map(|g| g.iter().map(|s| txns_by_sender.get(s).unwrap().len()).sum())
            .collect();
        // info!("group_sizes={:?}", &group_sizes);
        println!("max_group_size={:?}", group_sizes.iter().max());
        let (_, shard_ids_by_gid) = assign_tasks_to_workers(group_sizes, num_executor_shards);
        println!("assign_time={:?}", timer.elapsed());

        let timer = Instant::now();
        let mut txns_by_shard_id: Vec<Vec<Transaction>> = vec![vec![]; num_executor_shards];
        for (sender, txns) in txns_by_sender.into_iter() {
            let group_id = *group_ids_by_sender.get(&sender).unwrap();
            let shard_id = *shard_ids_by_gid.get(group_id).unwrap();
            txns_by_shard_id.get_mut(shard_id).unwrap().extend(txns);
        }
        println!("txns_by_shard_id_time={:?}", timer.elapsed());

        let timer = Instant::now();
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
        println!("build_ret_time={:?}", timer.elapsed());
        let worker_loads: Vec<usize> = ret.iter().map(|sbl| sbl.num_txns()).collect();
        println!("worker_loads={:?}", worker_loads);
        ret
    }
}
