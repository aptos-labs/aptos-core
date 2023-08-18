// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_gas_algebra::DynamicExpression;
use std::collections::{
    btree_map::Entry::{Occupied, Vacant},
    BTreeMap,
};

/// expand out the AST and collect only additive terms
///
/// ### Arguments
///
/// * `expr` - Root node of the AST
///
/// ### Example
///
/// * term -> num
/// * term -> var
/// * term -> term * term
/// * case 1: num -> Vec(num)
/// * case 2: var -> Vec(var)
/// * case 3: e1 * e2 -> t1 * t2
/// * case 4: e1 * e2 -> t1 + t2
pub fn expand_terms(expr: DynamicExpression) -> Vec<DynamicExpression> {
    let mut result: Vec<DynamicExpression> = Vec::new();
    match expr {
        DynamicExpression::GasValue { value } => {
            result.push(DynamicExpression::GasValue { value });
            result
        },
        DynamicExpression::GasParam { name } => {
            result.push(DynamicExpression::GasParam { name });
            result
        },
        DynamicExpression::Mul { left, right } => {
            let t1 = expand_terms(*left);
            let t2 = expand_terms(*right);
            let mut subresult: Vec<DynamicExpression> = Vec::new();
            for a_i in t1 {
                for b_j in &t2 {
                    match &a_i {
                        DynamicExpression::GasValue { .. } => {
                            subresult.push(DynamicExpression::Mul {
                                left: Box::new(a_i.clone()),
                                right: Box::new(b_j.clone()),
                            });
                        },
                        _ => {
                            subresult.push(DynamicExpression::Mul {
                                left: Box::new(b_j.clone()),
                                right: Box::new(a_i.clone()),
                            });
                        },
                    }
                }
            }
            result.extend(subresult.clone());
            result
        },
        DynamicExpression::Add { left, right } => {
            let t1 = expand_terms(*left);
            let t2 = expand_terms(*right);
            result.extend(t1.clone());
            result.extend(t2.clone());
            result
        },
    }
}

/// Simplify like-terms
///
/// ### Arguments
///
/// * `terms` - Vector of terms group by "+"
///
/// ### Example
///
/// (A + A) * (5 + 5) => 5A + 5A + 5A + 5A => 20A
pub fn aggregate_terms(terms: Vec<DynamicExpression>) -> Result<BTreeMap<String, u64>> {
    let mut map: BTreeMap<String, u64> = BTreeMap::new();
    for term in terms {
        match term {
            DynamicExpression::GasValue { value } => {
                // seeing a concrete quantity means that the user
                // isn't providing expressions in the GasExpression.
                // this makes calibration impossible and we should error
                if value != 0 {
                    return Err(anyhow!(
                        "Concrete quantity provided. Should be GasExpression."
                    ));
                }
            },
            DynamicExpression::GasParam { name } => match map.entry(name) {
                Occupied(mut entry) => {
                    *entry.get_mut() += 1;
                },
                Vacant(entry) => {
                    entry.insert(1);
                },
            },
            DynamicExpression::Mul { left, right } => {
                let mut key: String = String::new();
                let mut val: u64 = 0;

                if let DynamicExpression::GasParam { name } = *right {
                    key = name;
                }

                if let DynamicExpression::GasValue { value } = *left {
                    val = value;
                }

                if let Vacant(e) = map.entry(key.clone()) {
                    e.insert(val);
                } else {
                    map.entry(key).and_modify(|v| *v += val);
                }
            },
            _ => {},
        }
    }
    Ok(map)
}
