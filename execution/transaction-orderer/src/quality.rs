// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use crate::common::PTransaction;
use std::{collections::HashMap, hash::Hash, ops::Deref};

/// A simple cost function for a dependency between two transactions.
pub fn amortized_inverse_dependency_cost_function(a: f64) -> impl Fn(usize, usize) -> f64 {
    move |idx1, idx2| 1. / (a + idx1.abs_diff(idx2) as f64)
}

/// A function to estimate the quality of a transaction ordering.
///
/// The function takes a list of transactions and a dependency cost function.
/// Then, for each transaction, it finds the latest transaction that it depends on
/// and computes the cost of that dependency using the dependency cost function.
/// The total cost of the ordering is the sum of the costs of all closest dependencies.
pub fn order_total_cost<T, I, WF>(txns: I, cost_function: WF) -> f64
where
    T: PTransaction,
    T::Key: Eq + Hash + Clone,
    I: IntoIterator,
    I::Item: Deref<Target = T>,
    WF: Fn(usize, usize) -> f64,
{
    let mut latest_write = HashMap::<T::Key, usize>::new();
    let mut total_cost = 0.;

    for (idx, tx) in txns.into_iter().enumerate() {
        let max_dependency = tx.read_set().filter_map(|key| latest_write.get(key)).max();
        if let Some(&dep) = max_dependency {
            total_cost += cost_function(dep, idx);
        }

        for key in tx.write_set() {
            latest_write.insert(key.clone(), idx);
        }
    }

    total_cost
}
