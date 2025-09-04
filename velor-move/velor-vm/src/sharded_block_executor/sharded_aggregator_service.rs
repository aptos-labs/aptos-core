// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::aggr_overridden_state_view::TOTAL_SUPPLY_AGGR_BASE_VAL;
use velor_experimental_runtimes::thread_manager::optimal_min_len;
use velor_types::{
    state_store::{state_key::StateKey, StateView},
    transaction::TransactionOutput,
    write_set::TOTAL_SUPPLY_STATE_KEY,
};
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use std::{ops, sync::Arc};

pub fn get_state_value<S: StateView, T: DeserializeOwned>(
    state_key: &StateKey,
    state_view: &S,
) -> Option<T> {
    let value = state_view
        .get_state_value_bytes(state_key)
        .ok()?
        .map(move |value| bcs::from_bytes(&value));
    value.transpose().map_err(anyhow::Error::msg).unwrap()
}

/// This class ensures that deltas can use all 128 bits without having to let go of the sign bit for
/// cases where the delta is negative. That is, we don't have to use conversions to i128.
/// However, it does not handle overflow and underflow. That is, it will indicate to the caller of
/// the faulty logic with their usage of deltas.
#[derive(Clone, Copy)]
struct DeltaU128 {
    delta: u128,
    is_positive: bool,
}

impl DeltaU128 {
    pub fn get_delta(minuend: u128, subtrahend: u128) -> Self {
        if minuend >= subtrahend {
            Self {
                delta: minuend - subtrahend,
                is_positive: true,
            }
        } else {
            Self {
                delta: subtrahend - minuend,
                is_positive: false,
            }
        }
    }

    fn add_delta(self, other: u128) -> u128 {
        if self.is_positive {
            self.delta + other
        } else {
            other - self.delta
        }
    }
}

impl Default for DeltaU128 {
    fn default() -> Self {
        Self {
            delta: 0,
            is_positive: true,
        }
    }
}

impl ops::Add for DeltaU128 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // the deltas are both positive or both negative, we add the deltas and keep the sign
        if self.is_positive == rhs.is_positive {
            return Self {
                delta: self.delta + rhs.delta,
                is_positive: self.is_positive,
            };
        }

        // the deltas are of opposite signs, we subtract the smaller from the larger and keep the
        // sign of the larger
        let (pos, neg) = if self.is_positive {
            (self.delta, rhs.delta)
        } else {
            (rhs.delta, self.delta)
        };

        if pos >= neg {
            return Self {
                delta: pos - neg,
                is_positive: true,
            };
        }
        Self {
            delta: neg - pos,
            is_positive: false,
        }
    }
}

#[test]
fn test_delta_u128() {
    assert_eq!(DeltaU128::default().delta, 0);
    assert!(DeltaU128::default().is_positive);

    {
        // get positive delta
        let delta_pos = DeltaU128::get_delta(100, 50);
        assert_eq!(delta_pos.delta, 50);
        assert!(delta_pos.is_positive);

        // get negative delta
        let delta_neg = DeltaU128::get_delta(50, 100);
        assert_eq!(delta_neg.delta, 50);
        assert!(!delta_neg.is_positive);
    }

    {
        // test add_delta
        let delta1 = DeltaU128 {
            delta: 100,
            is_positive: true,
        };
        assert_eq!(delta1.add_delta(50), 150);

        let delta2 = DeltaU128 {
            delta: 50,
            is_positive: false,
        };
        assert_eq!(delta2.add_delta(50), 0);
    }

    {
        // test all cases for ops::Add
        let delta1 = DeltaU128 {
            delta: 100,
            is_positive: true,
        };
        let delta2 = DeltaU128 {
            delta: 50,
            is_positive: false,
        };
        let delta3 = DeltaU128 {
            delta: 100,
            is_positive: true,
        };
        // checks for [pos > neg]; [pos, pos]
        let mut delta_sum = delta1 + delta2 + delta3;
        assert_eq!(delta_sum.delta, 150);
        assert!(delta_sum.is_positive);

        let delta4 = DeltaU128 {
            delta: 500,
            is_positive: false,
        };
        let delta5 = DeltaU128 {
            delta: 200,
            is_positive: false,
        };
        // checks for [neg > pos]; [neg, neg]
        delta_sum = delta_sum + delta4 + delta5;
        assert_eq!(delta_sum.delta, 550);
        assert!(!delta_sum.is_positive);
    }
}

pub fn aggregate_and_update_total_supply<S: StateView>(
    sharded_output: &mut Vec<Vec<Vec<TransactionOutput>>>,
    global_output: &mut [TransactionOutput],
    state_view: &S,
    executor_thread_pool: Arc<rayon::ThreadPool>,
) {
    let num_shards = sharded_output.len();
    let num_rounds = sharded_output[0].len();

    // The first element is 0, which is the delta for shard 0 in round 0. +1 element will contain
    // the delta for the global shard
    let mut aggr_total_supply_delta = vec![DeltaU128::default(); num_shards * num_rounds + 1];

    // No need to parallelize this as the runtime is O(num_shards * num_rounds)
    // TODO: Get this from the individual shards while getting 'sharded_output'
    let mut aggr_ts_idx = 1;
    for round in 0..num_rounds {
        sharded_output.iter().for_each(|shard_output| {
            let mut curr_delta = DeltaU128::default();
            // Though we expect all the txn_outputs to have total_supply, there can be
            // exceptions like 'block meta' (first txn in the block) and 'chkpt info' (last txn
            // in the block) which may not have total supply. Hence we iterate till we find the
            // last txn with total supply.
            for txn in shard_output[round].iter().rev() {
                if let Some(last_txn_total_supply) = txn.write_set().get_total_supply() {
                    curr_delta =
                        DeltaU128::get_delta(last_txn_total_supply, TOTAL_SUPPLY_AGGR_BASE_VAL);
                    break;
                }
            }
            aggr_total_supply_delta[aggr_ts_idx] =
                curr_delta + aggr_total_supply_delta[aggr_ts_idx - 1];
            aggr_ts_idx += 1;
        });
    }

    // The txn_outputs contain 'txn_total_supply' with
    // 'CrossShardStateViewAggrOverride::total_supply_aggr_base_val' as the base value.
    // The actual 'total_supply_base_val' is in the state_view.
    // The 'delta' for the shard/round is in aggr_total_supply_delta[round * num_shards + shard_id + 1]
    // For every txn_output, we have to compute
    //      txn_total_supply = txn_total_supply - CrossShardStateViewAggrOverride::total_supply_aggr_base_val + total_supply_base_val + delta
    // While 'txn_total_supply' is u128, the intermediate computation can be negative. So we use
    // DeltaU128 to handle any intermediate underflow of u128.
    let total_supply_base_val: u128 = get_state_value(&TOTAL_SUPPLY_STATE_KEY, state_view).unwrap();
    let base_val_delta = DeltaU128::get_delta(total_supply_base_val, TOTAL_SUPPLY_AGGR_BASE_VAL);

    let aggr_total_supply_delta_ref = &aggr_total_supply_delta;
    // Runtime is O(num_txns), hence parallelized at the shard level and at the txns level.
    executor_thread_pool.scope(|_| {
        sharded_output
            .par_iter_mut()
            .enumerate()
            .for_each(|(shard_id, shard_output)| {
                for (round, txn_outputs) in shard_output.iter_mut().enumerate() {
                    let delta_for_round =
                        aggr_total_supply_delta_ref[round * num_shards + shard_id] + base_val_delta;
                    let num_txn_outputs = txn_outputs.len();
                    txn_outputs
                        .par_iter_mut()
                        .with_min_len(optimal_min_len(num_txn_outputs, 32))
                        .for_each(|txn_output| {
                            if let Some(txn_total_supply) =
                                txn_output.write_set().get_total_supply()
                            {
                                txn_output.update_total_supply(
                                    delta_for_round.add_delta(txn_total_supply),
                                );
                            }
                        });
                }
            });
    });

    let delta_for_global_shard = aggr_total_supply_delta[num_shards * num_rounds] + base_val_delta;
    let delta_for_global_shard_ref = &delta_for_global_shard;
    executor_thread_pool.scope(|_| {
        let num_txn_outputs = global_output.len();
        global_output
            .par_iter_mut()
            .with_min_len(optimal_min_len(num_txn_outputs, 32))
            .for_each(|txn_output| {
                if let Some(txn_total_supply) = txn_output.write_set().get_total_supply() {
                    txn_output.update_total_supply(
                        delta_for_global_shard_ref.add_delta(txn_total_supply),
                    );
                }
            });
    });
}
