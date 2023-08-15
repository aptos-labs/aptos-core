// Copyright © Aptos Foundation

use crate::v2::{
    counters::MISC_TIMERS_SECONDS, extract_and_sort, state::PartitionState, types::PreParedTxnIdx,
    PartitionerV2,
};
use aptos_logger::trace;
use aptos_types::block_executor::partitioner::{RoundId, TxnIndex};
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, IntoParallelRefIterator},
};
use std::{mem, sync::RwLock};

impl PartitionerV2 {
    /// Populate `state.finalized_txn_matrix` with txns flattened into a matrix (num_rounds by num_shards),
    /// in a way that avoid in-round cross-shard conflicts.
    pub(crate) fn partition_to_matrix(state: &mut PartitionState) {
        let _timer = MISC_TIMERS_SECONDS
            .with_label_values(&["partition_to_matrix"])
            .start_timer();

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

        let _timer = MISC_TIMERS_SECONDS
            .with_label_values(&["last_round"])
            .start_timer();

        if !state.partition_last_round {
            trace!("Merging txns after discarding stopped.");
            let last_round_txns: Vec<PreParedTxnIdx> =
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
    }

    /// Given some pre-partitioned txns, pull some off from each shard to avoid cross-shard conflict.
    /// The pulled off txns become the pre-partitioned txns for the next round.
    pub(crate) fn discarding_round(
        state: &mut PartitionState,
        round_id: RoundId,
        remaining_txns: Vec<Vec<PreParedTxnIdx>>,
    ) -> (Vec<Vec<PreParedTxnIdx>>, Vec<Vec<PreParedTxnIdx>>) {
        let _timer = MISC_TIMERS_SECONDS
            .with_label_values(&[format!("round_{round_id}").as_str()])
            .start_timer();

        let num_shards = remaining_txns.len();
        let mut discarded: Vec<RwLock<Vec<PreParedTxnIdx>>> = Vec::with_capacity(num_shards);
        let mut tentatively_accepted: Vec<RwLock<Vec<PreParedTxnIdx>>> =
            Vec::with_capacity(num_shards);
        let mut finally_accepted: Vec<RwLock<Vec<PreParedTxnIdx>>> = Vec::with_capacity(num_shards);
        for txns in remaining_txns.iter() {
            tentatively_accepted.push(RwLock::new(Vec::with_capacity(txns.len())));
            finally_accepted.push(RwLock::new(Vec::with_capacity(txns.len())));
            discarded.push(RwLock::new(Vec::with_capacity(txns.len())));
        }

        state.reset_min_discard_table();

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
                        let in_round_conflict_detected = state
                            .all_hints(txn_idx)
                            .into_iter()
                            .any(|key_idx| state.key_owned_by_another_shard(shard_id, key_idx));
                        //TODO: early stop.
                        if in_round_conflict_detected {
                            let sender = state.sender_idx(txn_idx);
                            state.update_min_discarded_txn_idx(sender, txn_idx);
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
                    txn_idxs.into_par_iter().for_each(|ori_txn_idx| {
                        let sender_idx = state.sender_idx(ori_txn_idx);
                        if ori_txn_idx
                            < state.min_discard(sender_idx).unwrap_or(PreParedTxnIdx::MAX)
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

        (
            extract_and_sort(finally_accepted),
            extract_and_sort(discarded),
        )
    }

    pub(crate) fn build_index_from_txn_matrix(state: &mut PartitionState) {
        let _timer = MISC_TIMERS_SECONDS
            .with_label_values(&["build_index_from_txn_matrix"])
            .start_timer();

        let num_rounds = state.finalized_txn_matrix.len();
        state.start_index_matrix = vec![vec![0; state.num_executor_shards]; num_rounds];
        let mut global_counter: TxnIndex = 0;
        for (round_id, row) in state.finalized_txn_matrix.iter().enumerate() {
            for (shard_id, txns) in row.iter().enumerate() {
                state.start_index_matrix[round_id][shard_id] = global_counter;
                global_counter += txns.len();
            }
        }

        state.new_txn_idxs = (0..state.num_txns()).map(|_tid| RwLock::new(0)).collect();

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
                                *state.new_txn_idxs[txn_idx].write().unwrap() =
                                    state.start_index_matrix[round_id][shard_id] + pos_in_sub_block;
                            });
                    });
            });
        });
    }
}
