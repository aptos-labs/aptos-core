// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_state_view::StateView;
use aptos_types::{
    state_store::state_key::StateKey, transaction::TransactionOutput,
    write_set::TOTAL_SUPPLY_STATE_KEY,
};
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use std::ops;

pub fn get_state_value<S: StateView, T: DeserializeOwned>(
    state_key: &StateKey,
    state_view: &S,
) -> Option<T> {
    let value = state_view
        .get_state_value_bytes(state_key)
        .ok()?
        .map(move |value| bcs::from_bytes(value.as_slice()));
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
        if self.is_positive == rhs.is_positive {
            Self {
                delta: self.delta + rhs.delta,
                is_positive: self.is_positive,
            }
        } else {
            let (pos, neg) = if self.is_positive {
                (self.delta, rhs.delta)
            } else {
                (rhs.delta, self.delta)
            };
            if pos >= neg {
                Self {
                    delta: pos - neg,
                    is_positive: true,
                }
            } else {
                Self {
                    delta: neg - pos,
                    is_positive: false,
                }
            }
        }
    }
}

pub fn aggregate_and_update_total_supply<S: StateView>(
    sharded_output: &mut Vec<Vec<Vec<TransactionOutput>>>,
    global_output: &mut [TransactionOutput],
    state_view: &S,
) {
    let num_shards = sharded_output.len();
    let num_rounds = sharded_output[0].len();
    let total_supply_base_val: u128 = get_state_value(&TOTAL_SUPPLY_STATE_KEY, state_view).unwrap();

    let mut aggr_total_supply_delta = vec![DeltaU128::default(); num_shards * num_rounds + 1];

    // No need to parallelize this as the runtime is O(num_shards)
    // TODO: Get this from the individual shards while getting 'sharded_output'
    sharded_output
        .iter()
        .enumerate()
        .for_each(|(shard_id, shard_output)| {
            for (round, txn_outputs) in shard_output.iter().enumerate() {
                for last_txn in txn_outputs.iter().rev() {
                    if let Some(last_txn_total_supply) =
                        last_txn.write_set().get_value(&TOTAL_SUPPLY_STATE_KEY)
                    {
                        aggr_total_supply_delta[round * num_shards + shard_id + 1] =
                            DeltaU128::get_delta(last_txn_total_supply, total_supply_base_val);
                        break;
                    }
                }
            }
        });

    for idx in 1..aggr_total_supply_delta.len() - 1 {
        aggr_total_supply_delta[idx + 1] =
            aggr_total_supply_delta[idx + 1] + aggr_total_supply_delta[idx];
    }

    // Runtime is O(num_txns), hence parallelized at the shard level and at the txns level.
    sharded_output
        .par_iter_mut()
        .enumerate()
        .for_each(|(shard_id, shard_output)| {
            for (round, txn_outputs) in shard_output.iter_mut().enumerate() {
                let delta_for_round = aggr_total_supply_delta[round * num_shards + shard_id];
                txn_outputs.par_iter_mut().for_each(|txn_output| {
                    if let Some(total_supply) =
                        txn_output.write_set().get_value(&TOTAL_SUPPLY_STATE_KEY)
                    {
                        txn_output.update_total_supply(delta_for_round.add_delta(total_supply));
                    }
                });
            }
        });

    let delta_for_global_shard = aggr_total_supply_delta[num_shards * num_rounds];
    global_output.par_iter_mut().for_each(|txn_output| {
        if let Some(total_supply) = txn_output.write_set().get_value(&TOTAL_SUPPLY_STATE_KEY) {
            txn_output.update_total_supply(delta_for_global_shard.add_delta(total_supply));
        }
    });
}
