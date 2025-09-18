// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::v2::{
    counters::MISC_TIMERS_SECONDS,
    extract_and_sort,
    state::PartitionState,
    types::{PrePartitionedTxnIdx, SenderIdx},
    PartitionerV2,
};
use aptos_logger::trace;
use aptos_metrics_core::TimerHelper;
use aptos_types::block_executor::partitioner::{RoundId, TxnIndex};
use dashmap::DashMap;
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, IntoParallelRefIterator},
};
use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
};

impl PartitionerV2 {
    /// Populate `state.finalized_txn_matrix` with txns flattened into a matrix (num_rounds by num_shards),
    /// in a way that avoid in-round cross-shard conflicts.
    pub(crate) fn remove_cross_shard_dependencies(state: &mut PartitionState) {
        let _timer = MISC_TIMERS_SECONDS.timer_with(&["remove_cross_shard_dependencies"]);

        let mut remaining_txns = mem::take(&mut state.pre_partitioned);
        assert_eq!(state.num_executor_shards, remaining_txns.len());

        let mut num_remaining_txns: usize;
        for round_id in 0..(state.num_rounds_limit - 1) {
            let (accepted, discarded) = Self::discarding_round(state, round_id, remaining_txns);
            state.finalized_txn_matrix.push(accepted);
            remaining_txns = discarded;
            num_remaining_txns = remaining_txns.iter().map(|ts| ts.len()).sum();

            if num_remaining_txns
                < ((1.0 - state.cross_shard_dep_avoid_threshold) * state.num_txns() as f32) as usize
            {
                break;
            }
        }

        let _timer = MISC_TIMERS_SECONDS.timer_with(&["last_round"]);

        if !state.partition_last_round {
            trace!("Merging txns after discarding stopped.");
            let last_round_txns: Vec<PrePartitionedTxnIdx> =
                remaining_txns.into_iter().flatten().collect();
            remaining_txns = vec![vec![]; state.num_executor_shards];
            remaining_txns[state.num_executor_shards - 1] = last_round_txns;
        }

        let last_round_id = state.finalized_txn_matrix.len();
        state.thread_pool.install(|| {
            (0..state.num_executor_shards)
                .into_par_iter()
                .for_each(|shard_id| {
                    remaining_txns[shard_id].par_iter().for_each(|&txn_idx| {
                        state.update_trackers_on_accepting(txn_idx, last_round_id, shard_id);
                    });
                });
        });
        state.finalized_txn_matrix.push(remaining_txns);
    }

    /// Given some pre-partitioned txns, pull some off from each shard to avoid cross-shard conflict.
    /// The pulled off txns become the pre-partitioned txns for the next round.
    pub(crate) fn discarding_round(
        state: &mut PartitionState,
        round_id: RoundId,
        remaining_txns: Vec<Vec<PrePartitionedTxnIdx>>,
    ) -> (
        Vec<Vec<PrePartitionedTxnIdx>>,
        Vec<Vec<PrePartitionedTxnIdx>>,
    ) {
        let _timer = MISC_TIMERS_SECONDS.timer_with(&[format!("round_{round_id}").as_str()]);

        let num_shards = remaining_txns.len();

        // Overview of the logic:
        // 1. Key conflicts are analyzed and a txn from `remaining_txns` either goes to `discarded` or `tentatively_accepted`.
        // 2. Relative orders of txns from the same sender are analyzed and a txn from `tentatively_accepted` either goes to `finally_accepted` or `discarded`.
        let mut discarded: Vec<RwLock<Vec<PrePartitionedTxnIdx>>> = Vec::with_capacity(num_shards);
        let mut tentatively_accepted: Vec<RwLock<Vec<PrePartitionedTxnIdx>>> =
            Vec::with_capacity(num_shards);
        let mut finally_accepted: Vec<RwLock<Vec<PrePartitionedTxnIdx>>> =
            Vec::with_capacity(num_shards);

        for txns in remaining_txns.iter() {
            tentatively_accepted.push(RwLock::new(Vec::with_capacity(txns.len())));
            finally_accepted.push(RwLock::new(Vec::with_capacity(txns.len())));
            discarded.push(RwLock::new(Vec::with_capacity(txns.len())));
        }

        // Initialize a table to keep track of the minimum discarded PrePartitionedTxnIdx.
        let min_discard_table: DashMap<SenderIdx, AtomicUsize> =
            DashMap::with_shard_amount(state.dashmap_num_shards);

        state.thread_pool.install(|| {
            // Move some txns to the next round (stored in `discarded`).
            // For those who remain in the current round (`tentatively_accepted`),
            // it's guaranteed to have no cross-shard conflicts.
            remaining_txns
                .into_iter()
                .enumerate()
                .collect::<Vec<_>>()
                .into_par_iter()
                .for_each(|(shard_id, txn_idxs)| {
                    txn_idxs.into_par_iter().for_each(|txn_idx| {
                        let ori_txn_idx = state.ori_idxs_by_pre_partitioned[txn_idx];
                        let mut in_round_conflict_detected = false;
                        let write_set = state.write_sets[ori_txn_idx].read().unwrap();
                        let read_set = state.read_sets[ori_txn_idx].read().unwrap();
                        for &key_idx in write_set.iter().chain(read_set.iter()) {
                            if state.key_owned_by_another_shard(shard_id, key_idx) {
                                in_round_conflict_detected = true;
                                break;
                            }
                        }

                        if in_round_conflict_detected {
                            let sender = state.sender_idx(ori_txn_idx);
                            min_discard_table
                                .entry(sender)
                                .or_insert_with(|| AtomicUsize::new(usize::MAX))
                                .fetch_min(txn_idx, Ordering::SeqCst);
                            discarded[shard_id].write().unwrap().push(txn_idx);
                        } else {
                            tentatively_accepted[shard_id]
                                .write()
                                .unwrap()
                                .push(txn_idx);
                        }
                    });
                });

            // Additional discarding to preserve relative txn order for the same sender.
            tentatively_accepted
                .into_iter()
                .enumerate()
                .collect::<Vec<_>>()
                .into_par_iter()
                .for_each(|(shard_id, txn_idxs)| {
                    let txn_idxs = mem::take(&mut *txn_idxs.write().unwrap());
                    txn_idxs.into_par_iter().for_each(|txn_idx| {
                        let ori_txn_idx = state.ori_idxs_by_pre_partitioned[txn_idx];
                        let sender_idx = state.sender_idx(ori_txn_idx);
                        let min_discarded = min_discard_table
                            .get(&sender_idx)
                            .map(|kv| kv.load(Ordering::SeqCst))
                            .unwrap_or(usize::MAX);
                        if txn_idx < min_discarded {
                            state.update_trackers_on_accepting(txn_idx, round_id, shard_id);
                            finally_accepted[shard_id].write().unwrap().push(txn_idx);
                        } else {
                            discarded[shard_id].write().unwrap().push(txn_idx);
                        }
                    });
                });
        });

        state.thread_pool.spawn(move || {
            drop(min_discard_table);
        });

        (
            extract_and_sort(finally_accepted),
            extract_and_sort(discarded),
        )
    }

    pub(crate) fn build_index_from_txn_matrix(state: &mut PartitionState) {
        let _timer = MISC_TIMERS_SECONDS.timer_with(&["build_index_from_txn_matrix"]);

        let num_rounds = state.finalized_txn_matrix.len();
        state.start_index_matrix = vec![vec![0; state.num_executor_shards]; num_rounds];
        let mut global_counter: TxnIndex = 0;
        for (round_id, row) in state.finalized_txn_matrix.iter().enumerate() {
            for (shard_id, txns) in row.iter().enumerate() {
                state.start_index_matrix[round_id][shard_id] = global_counter;
                global_counter += txns.len();
            }
        }

        state.final_idxs_by_pre_partitioned =
            (0..state.num_txns()).map(|_tid| RwLock::new(0)).collect();

        state.thread_pool.install(|| {
            (0..num_rounds).into_par_iter().for_each(|round_id| {
                (0..state.num_executor_shards)
                    .into_par_iter()
                    .for_each(|shard_id| {
                        let sub_block_size = state.finalized_txn_matrix[round_id][shard_id].len();
                        (0..sub_block_size)
                            .into_par_iter()
                            .for_each(|pos_in_sub_block| {
                                let txn_idx = state.finalized_txn_matrix[round_id][shard_id]
                                    [pos_in_sub_block];
                                *state.final_idxs_by_pre_partitioned[txn_idx]
                                    .write()
                                    .unwrap() =
                                    state.start_index_matrix[round_id][shard_id] + pos_in_sub_block;
                            });
                    });
            });
        });
    }
}
