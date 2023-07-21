// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::DynamicExpression;
use std::collections::BTreeMap;

/// expand out the AST and collect only terms
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
pub fn normalize(expr: DynamicExpression) -> Vec<DynamicExpression> {
    let mut result: Vec<DynamicExpression> = Vec::new();
    match expr {
        DynamicExpression::GasValue { value } => {
            result.push(DynamicExpression::GasValue { value });
            return result;
        },
        DynamicExpression::GasParam { name } => {
            result.push(DynamicExpression::GasParam { name });
            return result;
        },
        DynamicExpression::Mul { left, right } => {
            let t1 = normalize(*left);
            let t2 = normalize(*right);
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
            return result;
        },
        DynamicExpression::Add { left, right } => {
            let t1 = normalize(*left);
            let t2 = normalize(*right);
            result.extend(t1.clone());
            result.extend(t2.clone());
            return result;
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
pub fn collect_terms(terms: Vec<DynamicExpression>) -> BTreeMap<String, u64> {
    let mut map: BTreeMap<String, u64> = BTreeMap::new();
    for term in terms {
        match term {
            DynamicExpression::GasValue { value } => {
                let key: String = value.to_string();
                if !map.contains_key(&key) {
                    map.insert(key, value);
                } else {
                    map.entry(key).and_modify(|v| *v += value);
                }
            },
            DynamicExpression::GasParam { name } => {
                if !map.contains_key(&name) {
                    map.insert(name, 1);
                } else {
                    map.entry(name).and_modify(|v| *v += 1);
                }
            },
            DynamicExpression::Mul { left, right } => {
                let mut key: String = String::new();
                let mut val: u64 = 0;

                match *right {
                    DynamicExpression::GasParam { name } => {
                        key = name;
                    },
                    _ => {},
                }

                match *left {
                    DynamicExpression::GasValue { value } => {
                        val = value;
                    },
                    _ => {},
                }

                if !map.contains_key(&key) {
                    map.insert(key, val);
                } else {
                    map.entry(key).and_modify(|v| *v += val);
                }
            },
            _ => {},
        }
    }
    map
}
