// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use velor_gas_algebra::DynamicExpression;
use std::collections::{
    btree_map::Entry::{Occupied, Vacant},
    BTreeMap,
};

/// Expands a gas expression into a list of terms that add up to the original expression.
/// A term is an expression that does not contain the add operation.
///
/// Note that the result is NOT guaranteed to be simplified or in canonical form.
///
/// Here are some examples:
/// - `1 => [1]`
/// - `x => [x]`
/// - `(1 + 2)*x => [1*x, 2*x]`
/// - `5*(x + y) => [5*x, 5*y]`
/// - `(a + b)*(c + d) => [a*c, a*d, b*c, b*d]`
/// - `(1 + 2)*3*5 + 4 => [1*3*5, 2*3*5, 4]`
pub fn expand_terms(expr: DynamicExpression) -> Vec<DynamicExpression> {
    use DynamicExpression::{Add, GasParam, GasValue, Mul};

    match expr {
        GasValue { value } => {
            vec![GasValue { value }]
        },
        GasParam { name } => {
            vec![GasParam { name }]
        },
        Mul { left, right } => {
            let left_terms = expand_terms(*left);
            let right_terms = expand_terms(*right);

            let mut result = vec![];
            for left_term in &left_terms {
                for right_term in &right_terms {
                    match right_term {
                        // Attempt to place coefficients on the left side, but this is primarily
                        // for aesthetic reasons and does not provide a guarantee.
                        GasValue { .. } => result.push(Mul {
                            left: Box::new(right_term.clone()),
                            right: Box::new(left_term.clone()),
                        }),
                        _ => result.push(Mul {
                            left: Box::new(left_term.clone()),
                            right: Box::new(right_term.clone()),
                        }),
                    }
                }
            }
            result
        },
        Add { left, right } => {
            let mut result = expand_terms(*left);
            result.extend(expand_terms(*right));
            result
        },
    }
}

/// Takes a list of simple terms and combines the like-terms.
///
/// A simple term is a gas parameter multiplied by some coefficient. It can have a few forms:
/// - `coefficient * param`
/// - `param * coefficient`
/// - `param` (implicit coefficient of 1)
///
/// Zero terms (`x*0`, `0*x`, `0`) are ignored, while all other non-simple terms will result in an error.
///
/// Examples
/// - `x => { x: 1 }`
/// - `x*3 + 2*x => { x: 5 }`
/// - `0 + x + 2*y => { x: 1, y: 2 }`
pub fn aggregate_terms(terms: Vec<DynamicExpression>) -> Result<BTreeMap<String, u64>> {
    use DynamicExpression::{GasParam, GasValue, Mul};

    let mut map: BTreeMap<String, u64> = BTreeMap::new();
    for term in terms {
        match term {
            GasValue { value } => {
                // Seeing a concrete quantity here means that the VM or native functions do not have
                // some of its costs encoded abstractly using GasEpression.
                //
                // This makes calibration impossible and we should error.
                if value != 0 {
                    bail!(
                        "Expected simple term got non-zero concrete value {}.",
                        value
                    );
                }
            },
            GasParam { name } => match map.entry(name) {
                Occupied(mut entry) => {
                    *entry.get_mut() += 1;
                },
                Vacant(entry) => {
                    entry.insert(1);
                },
            },
            Mul { left, right } => match (*left, *right) {
                (GasParam { name }, GasValue { value })
                | (GasValue { value }, GasParam { name }) => {
                    if value != 0 {
                        match map.entry(name) {
                            Vacant(entry) => {
                                entry.insert(value);
                            },
                            Occupied(entry) => {
                                *entry.into_mut() += value;
                            },
                        }
                    }
                },
                (left, right) => bail!("Expected simple term got {:?} * {:?}.", left, right),
            },
            term => bail!("Expected simple term got {:?}.", term),
        }
    }
    Ok(map)
}
