// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::math::{
    add_gas_formula_to_coefficient_matrix, add_running_time_to_constant_matrix,
    compute_least_square_solutions, find_linearly_dependent_variables, find_outliers,
};
use crate::math_interface::generic_map;
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
    equation_names: Vec<String>,
) {
    let lss = compute_least_square_solutions(coeff_matrix, const_matrix);
    if lss.is_ok() {
        let mut x_hat = lss.unwrap();

        let map = generic_map(input.clone());
        let keys: Vec<String> = map.keys().map(|key| key.to_string()).collect();

        let nrows = x_hat.nrows();
        let ncols = x_hat.ncols();
        let mut i = 0;
        let mut j = 0;
        println!("where the gas parameter values are:\n");
        while i < nrows {
            while j < ncols {
                println!("{} {}", x_hat[(i, j)], keys[i]);
                j += 1;
            }
            i += 1;
            j = 0;
        }

        // TODO: error handling with division zero that bubbles up
        report_outliers(&mut x_hat, coeff_matrix, const_matrix, equation_names);
    } else {
        report_undetermined_gas_params(input, coeff_matrix, const_matrix, equation_names);
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
    x_hat: &mut DMatrix<f64>,
    coeff_matrix: &mut DMatrix<f64>,
    const_matrix: &mut DMatrix<f64>,
    equation_names: Vec<String>,
) {
    let outliers = find_outliers(x_hat, coeff_matrix, const_matrix).expect("should unwrap");

    println!("\noutliers are (times are in microseconds):\n");
    for (idx, cr, ar, err) in outliers {
        println!(
            "- {} | Computed {}ms vs. Actual {}ms | Error {}\n",
            equation_names[idx],
            cr,
            format!("{:.3}", ar),
            format!("{:.3}", err)
        );
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
    equation_names: Vec<String>,
) {
    let map = generic_map(input);
    let keys: Vec<String> = map.keys().map(|key| key.to_string()).collect();

    let result = find_linearly_dependent_variables(coeff_matrix, const_matrix);
    if result.is_err() {
        println!("free variables are:\n");
        let pivot_columns = result.unwrap_err();
        for col in pivot_columns {
            let gas_param = &keys[col];
            println!("- gas parameter: {}\n", gas_param);
        }
    } else {
        println!("linearly dependent variables are:\n");
        let linear_combos = result.unwrap();
        for (eq, gas_param) in linear_combos {
            let eq_name = &equation_names[eq];
            let gas_param_name = &keys[gas_param];
            println!("- {} | gas parameter: {}\n", eq_name, gas_param_name);
        }
    }
}
