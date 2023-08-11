// Copyright © Aptos Foundation

use crate::v2::{extract_and_sort, types::OriginalTxnIdx, PartitionState, PartitionerV2};
use aptos_logger::trace;
use aptos_types::block_executor::partitioner::RoundId;
use dashmap::DashMap;
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, IntoParallelRefIterator},
};
use std::{mem, sync::RwLock};

impl PartitionerV2 {
    pub(crate) fn flatten_to_rounds(state: &mut PartitionState) {
        let mut remaining_txns = mem::take(&mut state.pre_partitioned);
        assert_eq!(state.num_executor_shards, remaining_txns.len());

        let mut num_remaining_txns = usize::MAX;
        for round_id in 0..(state.num_rounds_limit - 1) {
            let (accepted, discarded) = Self::discarding_round(state, round_id, remaining_txns);
            state.finalized_txn_matrix.push(accepted);
            remaining_txns = discarded;
            num_remaining_txns = remaining_txns.iter().map(|ts| ts.len()).sum();

            if num_remaining_txns < state.avoid_pct as usize * state.num_txns() / 100 {
                break;
            }
        }

        if state.merge_discarded {
            trace!("Merging txns after discarding stopped.");
            let last_round_txns: Vec<OriginalTxnIdx> =
                remaining_txns.into_iter().flatten().collect();
            remaining_txns = vec![vec![]; state.num_executor_shards];
            remaining_txns[state.num_executor_shards - 1] = last_round_txns;
        }

        let last_round_id = state.finalized_txn_matrix.len();
        state.thread_pool.install(|| {
            (0..state.num_executor_shards)
                .into_par_iter()
                .for_each(|shard_id| {
                    remaining_txns[shard_id]
                        .par_iter()
                        .for_each(|&ori_txn_idx| {
                            state.update_trackers_on_accepting(
                                ori_txn_idx,
                                last_round_id,
                                shard_id,
                            );
                        });
                });
        });
        state.finalized_txn_matrix.push(remaining_txns);

        (state.start_index_matrix, state.new_txn_idxs) =
            state.build_new_index_tables(&state.finalized_txn_matrix);
    }

    /// Given some pre-partitioned txns, pull some off from each shard to avoid cross-shard conflict.
    /// The pulled off txns become the pre-partitioned txns for the next round.
    pub(crate) fn discarding_round(
        state: &mut PartitionState,
        round_id: RoundId,
        remaining_txns: Vec<Vec<OriginalTxnIdx>>,
    ) -> (Vec<Vec<OriginalTxnIdx>>, Vec<Vec<OriginalTxnIdx>>) {
        let num_shards = remaining_txns.len();
        let mut discarded: Vec<RwLock<Vec<OriginalTxnIdx>>> = Vec::with_capacity(num_shards);
        let mut potentially_accepted: Vec<RwLock<Vec<OriginalTxnIdx>>> =
            Vec::with_capacity(num_shards);
        let mut finally_accepted: Vec<RwLock<Vec<OriginalTxnIdx>>> = Vec::with_capacity(num_shards);
        for txns in remaining_txns.iter() {
            potentially_accepted.push(RwLock::new(Vec::with_capacity(txns.len())));
            finally_accepted.push(RwLock::new(Vec::with_capacity(txns.len())));
            discarded.push(RwLock::new(Vec::with_capacity(txns.len())));
        }

        state.min_discards_by_sender = DashMap::new();

        state.thread_pool.install(|| {
            (0..state.num_executor_shards)
                .into_par_iter()
                .for_each(|shard_id| {
                    remaining_txns[shard_id].par_iter().for_each(|&txn_idx| {
                        let in_round_conflict_detected =
                            state.all_hints(txn_idx).iter().any(|&key_idx| {
                                state.shard_is_currently_follower_for_key(shard_id, key_idx)
                            });
                        if in_round_conflict_detected {
                            let sender = state.sender_idx(txn_idx);
                            state.update_min_discarded_txn_idx(sender, txn_idx);
                            discarded[shard_id].write().unwrap().push(txn_idx);
                        } else {
                            potentially_accepted[shard_id]
                                .write()
                                .unwrap()
                                .push(txn_idx);
                        }
                    });
                });
        });

        state.thread_pool.install(|| {
            (0..num_shards).into_par_iter().for_each(|shard_id| {
                potentially_accepted[shard_id]
                    .read()
                    .unwrap()
                    .par_iter()
                    .for_each(|&ori_txn_idx| {
                        let sender_idx = state.sender_idx(ori_txn_idx);
                        if ori_txn_idx
                            < state.min_discard(sender_idx).unwrap_or(OriginalTxnIdx::MAX)
                        {
                            state.update_trackers_on_accepting(ori_txn_idx, round_id, shard_id);
                            finally_accepted[shard_id]
                                .write()
                                .unwrap()
                                .push(ori_txn_idx);
                        } else {
                            discarded[shard_id].write().unwrap().push(ori_txn_idx);
                        }
                    });
            });
        });

        let ret = (
            extract_and_sort(finally_accepted),
            extract_and_sort(discarded),
        );
        let min_discards_by_sender = mem::take(&mut state.min_discards_by_sender);
        state.thread_pool.spawn(move || {
            drop(remaining_txns);
            drop(potentially_accepted);
            drop(min_discards_by_sender);
        });
        ret
    }
}
