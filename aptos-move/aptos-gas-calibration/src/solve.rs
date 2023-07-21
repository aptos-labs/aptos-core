// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::math::{
    add_gas_formula_to_coefficient_matrix, add_running_time_to_constant_matrix,
    compute_least_square_solutions, find_free_variables, find_outliers,
};
use crate::math_interface::{convert_to_generic_map, generic_map};
use nalgebra::DMatrix;
use std::collections::BTreeMap;

/// wrapper function to build a coefficient matrix
///
/// ### Arguments
///
/// * `input` - Collection of like-terms
/// * `nrows` - Number of rows
/// * `ncols` - Number of cols
pub fn build_coefficient_matrix(input: Vec<Vec<f64>>, nrows: usize, ncols: usize) -> DMatrix<f64> {
    let mut coeff_matrix = DMatrix::<f64>::zeros(nrows, ncols);
    for (idx, eq) in input.iter().enumerate() {
        add_gas_formula_to_coefficient_matrix(idx, ncols, eq, &mut coeff_matrix);
    }
    // println!("coeff: {}\n", coeff_matrix);
    coeff_matrix
}

/// wrapper function to build a constant matrix
///
/// ### Arguments
///
/// * `input` - Collection of like-terms
/// * `nrows` - Number of rows
/// * `ncols` - Number of cols
pub fn build_constant_matrix(input: Vec<u128>, nrows: usize, ncols: usize) -> DMatrix<f64> {
    let mut const_matrix = DMatrix::<f64>::zeros(nrows, ncols);
    for (idx, run_time) in input.iter().enumerate() {
        add_running_time_to_constant_matrix(idx, *run_time as f64, &mut const_matrix);
    }
    // println!("const: {}\n", const_matrix);
    const_matrix
}

/// compute the least squares solution
///
/// ### Arguments
///
/// * `input` - Collection of like-terms
/// * `coeff_matrix` - Coefficient Matrix
/// * `const_matrix` - Constant Matrix
pub fn solve(
    input: Vec<BTreeMap<String, u64>>,
    coeff_matrix: &mut DMatrix<f64>,
    const_matrix: &mut DMatrix<f64>,
) {
    let lss = compute_least_square_solutions(coeff_matrix, const_matrix);
    if lss.is_ok() {
        let mut x_hat = lss.unwrap();

        let map = generic_map(input);
        let keys: Vec<String> = map.keys().map(|key| key.to_string()).collect();
        //println!("gas params: {:?}\n", keys);

        let nrows = x_hat.nrows();
        let ncols = x_hat.ncols();
        let mut i = 0;
        let mut j = 0;
        println!("x_hat solutions:\n");
        while i < nrows {
            while j < ncols {
                println!("{} {}", x_hat[(i, j)], keys[i]);
                j += 1;
            }
            i += 1;
            j = 0;
        }

        //println!("x_hat solutions: {}\n", x_hat);

        // TODO: error handling with division zero that bubbles up
        //report_outliers(input, &mut x_hat, coeff_matrix, const_matrix);
    } else {
        report_undetermined_gas_params(input, coeff_matrix, const_matrix);
    }
}

/// determine the outliers after computing least squares
///
/// ### Arguments
///
/// * `input` - Collection of like-terms
/// * `x_hat` - Least squares solution
/// * `coeff_matrix` - Coefficient Matrix
/// * `const_matrix` - Constant Matrix
fn report_outliers(
    input: Vec<BTreeMap<String, u64>>,
    x_hat: &mut DMatrix<f64>,
    coeff_matrix: &mut DMatrix<f64>,
    const_matrix: &mut DMatrix<f64>,
) {
    let outliers = find_outliers(x_hat, coeff_matrix, const_matrix).expect("should unwrap");

    let equations = convert_to_generic_map(input);

    println!("outliers are:\n");
    for (x, y) in outliers {
        let equation = &equations[x];
        let keys: Vec<String> = equation.keys().map(|key| key.to_string()).collect();
        println!("- gas parameter: {} in equation {}\n", keys[y], x);
    }
}

/// find the gas params that could not be determined if the system
/// was not solvable.
///
/// ### Arguments
///
/// * `input` - Collection of like-terms
/// * `coeff_matrix` - Coefficient Matrix
/// * `const_matrix` - Constant Matrix
fn report_undetermined_gas_params(
    input: Vec<BTreeMap<String, u64>>,
    coeff_matrix: &mut DMatrix<f64>,
    const_matrix: &mut DMatrix<f64>,
) {
    let free_variables = find_free_variables(coeff_matrix, const_matrix);

    let map = generic_map(input);
    let keys: Vec<String> = map.keys().map(|key| key.to_string()).collect();
    //println!("gas params: {:?}\n", keys);

    println!("free variables are:\n");
    for col in free_variables {
        let gas_param = &keys[col];
        println!("- gas parameter: {}\n", gas_param);
    }
}
