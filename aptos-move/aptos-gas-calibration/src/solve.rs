// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    math::{
        add_gas_formula_to_coefficient_matrix, add_running_time_to_constant_matrix,
        compute_least_square_solutions, find_linearly_dependent_variables,
        get_computed_time_and_outliers,
    },
    math_interface::generic_map,
};
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
pub fn least_squares(
    input: Vec<BTreeMap<String, u64>>,
    coeff_matrix: &mut DMatrix<f64>,
    const_matrix: &mut DMatrix<f64>,
    equation_names: Vec<String>,
) {
    let lss = compute_least_square_solutions(coeff_matrix, const_matrix);
    if let Ok(answer) = lss {
        let mut x_hat = answer;

        let map = generic_map(input.clone());
        let keys: Vec<String> = map.keys().map(|key| key.to_string()).collect();

        let nrows = x_hat.nrows();
        let ncols = x_hat.ncols();
        println!("where the gas parameter values are (microsecond per instruction):\n");
        for i in 0..nrows {
            for j in 0..ncols {
                println!("{} {}", x_hat[(i, j)], keys[i]);
            }
        }

        // TODO: error handling with division zero that bubbles up
        let computed_time_and_outliers =
            get_computed_time_and_outliers(&mut x_hat, coeff_matrix, const_matrix)
                .expect("Failed: should unwrap, possibly division by zero");

        report_computed_times(&equation_names, &computed_time_and_outliers);

        report_outliers(&equation_names, &computed_time_and_outliers);
    } else {
        report_undetermined_gas_params(input, coeff_matrix, const_matrix);
    }
}

/// display the computed running times to the user after computing least squares
///
/// ### Arguments
///
/// * `input` - Collection of like-terms
/// * `x_hat` - Least squares solution
/// * `coeff_matrix` - Coefficient Matrix
/// * `const_matrix` - Constant Matrix
fn report_computed_times(
    equation_names: &[String],
    actual_times: &Vec<(usize, f64, f64, f64, bool)>,
) {
    println!("\nComputed running times are:\n");
    for (idx, cr, ar, err, is_outlier) in actual_times {
        if *is_outlier {
            continue;
        };
        println!(
            "- {} | Computed {}µs vs. Actual {}µs | Error {}\n",
            equation_names[*idx],
            cr,
            format_args!("{:.3}", ar),
            format_args!("{:.3}", err)
        );
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
fn report_outliers(equation_names: &[String], outliers: &Vec<(usize, f64, f64, f64, bool)>) {
    println!("\nOutliers are:\n");
    for (idx, cr, ar, err, is_outlier) in outliers {
        if !is_outlier {
            continue;
        };
        println!(
            "- {} | Computed {}µs vs. Actual {}µs | Error {}\n",
            equation_names[*idx],
            cr,
            format_args!("{:.3}", ar),
            format_args!("{:.3}", err)
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
) {
    let map = generic_map(input);
    let keys: Vec<String> = map.keys().map(|key| key.to_string()).collect();

    let result = find_linearly_dependent_variables(coeff_matrix, const_matrix, keys.clone());
    match result {
        Ok(linear_combos) => {
            println!("linearly dependent variables are:\n");
            for gas_param in linear_combos {
                println!("- gas parameter: {}\n", gas_param);
            }
        },
        Err(pivot_columns) => {
            println!("free variables are:\n");
            for col in pivot_columns {
                let gas_param = &keys[col];
                println!("- gas parameter: {}\n", gas_param);
            }
        },
    }
}
