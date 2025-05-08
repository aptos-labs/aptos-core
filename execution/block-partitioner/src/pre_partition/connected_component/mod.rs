// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pre_partition::PrePartitioner,
    v2::{
        load_balance::longest_processing_time_first,
        state::PartitionState,
        types::{OriginalTxnIdx, PrePartitionedTxnIdx},
        union_find::UnionFind,
    },
};
use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::{AtomicUsize, Ordering},
};

/// A `PrePartitioner` used in `PartitionerV2` that tries to group conflicting txns together (but a group size limit applied),
/// then assign the groups to the shards using Longest-processing-time-first (LPT) scheduling.
/// <https://en.wikipedia.org/wiki/Longest-processing-time-first_scheduling>
///
/// Note that the number of groups varies depending on the block:
/// - if the conflict level is high and group size limit is loose, we may have only 1 group;
/// - if the all txns are independent, each will be in its own group.
///
/// The group size limit is controlled by parameter `load_imbalance_tolerance` in the following way:
/// if `block_size=100, num_shards=10, load_imbalance_tolerance=2.0`,
/// then the size of a conflicting txn group is not allowed to exceed 100/10*2.0 = 20.
/// This fact, combined with the LPT algorithm, guarantees that shard load will not exceed 20.
pub struct ConnectedComponentPartitioner {
    pub load_imbalance_tolerance: f32,
}

impl PrePartitioner for ConnectedComponentPartitioner {
    fn pre_partition(
        &self,
        state: &PartitionState,
    ) -> (
        Vec<OriginalTxnIdx>,
        Vec<PrePartitionedTxnIdx>,
        Vec<Vec<PrePartitionedTxnIdx>>,
    ) {
        // Union-find.
        // Each sender/state key initially in its own set.
        // For every declared storage access to key `k` by a txn from sender `s`, merge the set of `k` and that of `s`.
        let num_senders = state.num_senders();
        let num_keys = state.num_keys();
        let mut uf = UnionFind::new(num_senders + num_keys);
        for txn_idx in 0..state.num_txns() {
            let sender_idx = state.sender_idx(txn_idx);
            let write_set = state.write_sets[txn_idx].read().unwrap();
            for &key_idx in write_set.iter() {
                let key_idx_in_uf = num_senders + key_idx;
                uf.union(key_idx_in_uf, sender_idx);
            }
        }
        // NOTE: union-find result is NOT deterministic. But the following step can fix it.

        // Entities & relations involved in the following processing.
        //
        // txn-0 txn-7 txn-9     txn-1 txn-2 txn-3 txn-4 txn-5 txn-6 txn-8
        //      \  |  /               \    \   |     |     |   /    /
        //       \ | /                  \   |  |     |     |  |  /
        // conflicting-set-0                conflicting-set-1
        //      /      \                  /  |         |     \
        //     /        \               /    |         |      \
        // txn-grp-0 txn-grp-1  txn-grp-2 txn-grp-3 txn-grp-4 txn-grp-5
        //         \        \         \  /          /         /
        //          \        \         \/          /       /
        //            \       \        /\         /     /
        //               \     \     /    \      /  /
        //                  Shard-0         Shard-1

        // Prepare `txns_by_set`: a mapping from a conflicting set to its txns.
        let mut txns_by_set: Vec<VecDeque<OriginalTxnIdx>> = Vec::new();
        let mut set_idx_registry: HashMap<usize, usize> = HashMap::new();
        let set_idx_counter = AtomicUsize::new(0);
        for ori_txn_idx in 0..state.num_txns() {
            let sender_idx = state.sender_idx(ori_txn_idx);
            let uf_set_idx = uf.find(sender_idx);
            let set_idx = set_idx_registry.entry(uf_set_idx).or_insert_with(|| {
                txns_by_set.push(VecDeque::new());
                set_idx_counter.fetch_add(1, Ordering::SeqCst)
            });
            txns_by_set[*set_idx].push_back(ori_txn_idx);
        }

        // Calculate txn group size limit.
        let group_size_limit = ((state.num_txns() as f32) * self.load_imbalance_tolerance
            / (state.num_executor_shards as f32))
            .ceil() as usize;

        // Prepare `group_metadata`, a group_metadata (i, r) will later be converted to a real group that takes `r` txns from set `i`.
        // NOTE: If we create actual txn groups now and then do load-balanced scheduling, we break the relative order of txns from the same sender.
        // The workaround is to only fix the group set and their sizes for now, then schedule, and materialize the txn groups at the very end (when assigning groups to shards).
        let group_metadata: Vec<(usize, usize)> = txns_by_set
            .iter()
            .enumerate()
            .flat_map(|(set_idx, txns)| {
                let num_chunks = txns.len().div_ceil(group_size_limit);
                let mut ret = vec![(set_idx, group_size_limit); num_chunks];
                let last_chunk_size = txns.len() - group_size_limit * (num_chunks - 1);
                ret[num_chunks - 1] = (set_idx, last_chunk_size);
                ret
            })
            .collect();

        // Assign groups to shards using longest-processing-time first scheduling.
        let tasks: Vec<u64> = group_metadata
            .iter()
            .map(|(_, size)| (*size) as u64)
            .collect();
        let (_longest_pole, shards_by_group) =
            longest_processing_time_first(&tasks, state.num_executor_shards);

        // Prepare `groups_by_shard`: a mapping from a shard to the txn groups assigned to it.
        let mut groups_by_shard: Vec<Vec<usize>> = vec![vec![]; state.num_executor_shards];
        for (group_id, shard_id) in shards_by_group.into_iter().enumerate() {
            groups_by_shard[shard_id].push(group_id);
        }

        let mut ori_txns_idxs_by_shard: Vec<Vec<OriginalTxnIdx>> =
            vec![vec![]; state.num_executor_shards];
        for (shard_id, group_ids) in groups_by_shard.into_iter().enumerate() {
            for group_id in group_ids.into_iter() {
                let (set_id, amount) = group_metadata[group_id];
                for _ in 0..amount {
                    let ori_txn_idx = txns_by_set[set_id].pop_front().unwrap();
                    ori_txns_idxs_by_shard[shard_id].push(ori_txn_idx);
                }
            }
        }

        // Prepare `ori_txn_idxs` and `start_txn_idxs_by_shard`.
        let mut start_txn_idxs_by_shard = vec![0; state.num_executor_shards];
        let mut ori_txn_idxs = vec![0; state.num_txns()];
        let mut pre_partitioned_txn_idx = 0;
        for (shard_id, txn_idxs) in ori_txns_idxs_by_shard.iter().enumerate() {
            start_txn_idxs_by_shard[shard_id] = pre_partitioned_txn_idx;
            for &i0 in txn_idxs {
                ori_txn_idxs[pre_partitioned_txn_idx] = i0;
                pre_partitioned_txn_idx += 1;
            }
        }

        // Prepare `pre_partitioned`.
        let pre_partitioned = (0..state.num_executor_shards)
            .map(|shard_id| {
                let start = start_txn_idxs_by_shard[shard_id];
                let end: PrePartitionedTxnIdx = if shard_id == state.num_executor_shards - 1 {
                    state.num_txns()
                } else {
                    start_txn_idxs_by_shard[shard_id + 1]
                };
                (start..end).collect()
            })
            .collect();

        state.thread_pool.spawn(move || {
            drop(txns_by_set);
            drop(set_idx_registry);
            drop(group_metadata);
            drop(tasks);
            drop(ori_txns_idxs_by_shard);
        });

        (ori_txn_idxs, start_txn_idxs_by_shard, pre_partitioned)
    }
}

pub mod config;
