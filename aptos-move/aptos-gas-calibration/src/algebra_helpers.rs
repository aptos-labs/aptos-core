// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::Expression;
use std::collections::BTreeMap;

/*
 * @notice: expand out the AST and collect only terms
 * @param expr: Root node of the AST
 * @return result: Vector of terms grouped by "+"
 * @dev:
 *      term -> num
 *      term -> var
 *      term -> term * term
 *      case 1: num -> Vec[num]
 *      case 2: var -> Vec[var]
 *      case 3: e1 * e2 -> t1 * t2
 *      case 4: e1 * e2 -> t1 + t2
 */
pub fn normalize(expr: Expression) -> Vec<Expression> {
    let mut result: Vec<Expression> = Vec::new();
    match expr {
        Expression::GasValue { value } => {
            result.push(Expression::GasValue { value });
            return result;
        },
        Expression::GasParam { name } => {
            result.push(Expression::GasParam { name });
            return result;
        },
        Expression::Mul { left, right } => {
            let t1 = normalize(*left);
            let t2 = normalize(*right);
            let mut subresult: Vec<Expression> = Vec::new();
            for a_i in t1 {
                for b_j in &t2 {
                    match a_i.clone() {
                        Expression::GasValue { .. } => {
                            subresult.push(Expression::Mul {
                                left: Box::new(a_i.clone()),
                                right: Box::new(b_j.clone()),
                            });
                        },
                        _ => {
                            subresult.push(Expression::Mul {
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
        Expression::Add { left, right } => {
            let t1 = normalize(*left);
            let t2 = normalize(*right);
            result.extend(t1.clone());
            result.extend(t2.clone());
            return result;
        },
    }
}

/*
 * @notice: Simplify like-terms
 * @param terms: Vector of terms group by "+"
 * @return map: BTreeMap of the simplified expressions
 * @dev: (A + A) * (5 + 5) => 5A + 5A + 5A + 5A => 20A
 */
pub fn collect_terms(terms: Vec<Expression>) -> BTreeMap<String, u64> {
    let mut map: BTreeMap<String, u64> = BTreeMap::new();
    for term in terms {
        match term {
            Expression::GasValue { value } => {
                let key: String = value.to_string();
                if !map.contains_key(&key) {
                    map.insert(key, value);
                } else {
                    map.entry(key).and_modify(|v| *v += value);
                }
            },
            Expression::GasParam { name } => {
                if !map.contains_key(&name) {
                    map.insert(name, 1);
                } else {
                    map.entry(name).and_modify(|v| *v += 1);
                }
            },
            Expression::Mul { left, right } => {
                let mut key: String = String::new();
                let mut val: u64 = 0;

                match *right {
                    Expression::GasParam { name } => {
                        key = name;
                    },
                    _ => {},
                }

                match *left {
                    Expression::GasValue { value } => {
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
