// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use float_cmp::approx_eq;
use nalgebra::{self, DMatrix};
use std::ops::{Div, Mul};

const MARGIN_OF_ERROR: f64 = 0.2;

/// Add a gas formula to the coefficient matrix
///
/// ### Arguments
///
/// * `idx` - Keeps track of which row to edit
/// * `ncols` - Max number of columns
/// * `formula` - The gas formula to add
/// * `coefficient_matrix` - The Matrix we want to edit
pub fn add_gas_formula_to_coefficient_matrix(
    idx: usize,
    ncols: usize,
    formula: &[f64],
    coefficient_matrix: &mut DMatrix<f64>,
) {
    let mut j = 0;
    while j < ncols {
        coefficient_matrix[(idx, j)] = formula[j];
        j += 1;
    }
}

/// Add the running time corresponding to the gas formula to
/// constant matrix.
///
/// ### Arguments
///
/// * `idx` - Keeps track of which row to edit
/// * `running_time` - The running time w.r.t the gas formula
/// * `constant_matrix` - The Matrix we want to edit
pub fn add_running_time_to_constant_matrix(
    idx: usize,
    running_time: f64,
    constant_matrix: &mut DMatrix<f64>,
) {
    constant_matrix[(idx, 0)] = running_time;
}

/// Join coefficient and constant matrix to make augmented
///
/// ### Arguments
///
/// * `augmented_matrix` - Matrix to join coefficient and constant
/// * `coefficient_matrix` - Matrix for gas formula
/// * `constant_matrix` - Matrix for running time w.r.t gas formula
fn create_augmented_matrix(
    augmented_matrix: &mut DMatrix<f64>,
    coefficient_matrix: &mut DMatrix<f64>,
    constant_matrix: &mut DMatrix<f64>,
) {
    let mut i = 0;
    let mut j = 0;

    let nrows = augmented_matrix.nrows();
    let ncols = coefficient_matrix.ncols();
    while i < nrows {
        while j < ncols {
            augmented_matrix[(i, j)] = coefficient_matrix[(i, j)];
            j += 1;
        }
        i += 1;
        j = 0;
    }

    i = 0;
    while i < nrows {
        augmented_matrix[(i, ncols)] = constant_matrix[(i, 0)];
        i += 1;
    }
}

/// Compute least squares
///
/// ### Arguments
///
/// * `A` - Coefficient matrix
/// * `b` - Constant matrix
#[allow(non_snake_case)]
pub fn compute_least_square_solutions(
    A: &mut DMatrix<f64>,
    b: &mut DMatrix<f64>,
) -> Result<DMatrix<f64>, String> {
    let A_T = A.transpose();
    let A_TA = A_T.clone().mul(A.clone());
    let A_Tb = A_T.clone().mul(b.clone());

    if !A_TA.is_invertible() {
        return Err("cannot invert A_TA matrix".to_string());
    }

    let inverse = A_TA.try_inverse().expect("inverse should work");
    let x_hat = inverse.mul(&A_Tb);
    Ok(x_hat)
}

/// Find all free variables / linear dependent combinations
///
/// ### Arguments
///
/// * `A` - Coefficient matrix
/// * `b` - Constant matrix
#[allow(non_snake_case)]
pub fn find_linearly_dependent_variables(
    A: &mut DMatrix<f64>,
    b: &mut DMatrix<f64>,
    gas_params: Vec<String>,
) -> Result<Vec<String>, Vec<usize>> {
    let mut aug_matrix = DMatrix::<f64>::zeros(A.nrows(), A.ncols() + 1);
    create_augmented_matrix(&mut aug_matrix, A, b);
    rref(&mut aug_matrix);

    /*let is_ones = validate_rref_rows_are_one(&mut aug_matrix);
    if is_ones {
        return Err(find_pivot_columns(&mut aug_matrix));
    }*/

    let linearly_independent = find_linear_independent_variables(&mut aug_matrix);
    let mut linearly_dependent = Vec::new();
    for (idx, gas_param) in gas_params.into_iter().enumerate() {
        if !linearly_independent.contains(&idx) {
            linearly_dependent.push(gas_param)
        }
    }
    Ok(linearly_dependent)
}

/// We use the Least Squares solution to input into the LHS to get what we
/// call as the Computed Time. We compare this against the LHS (the Actual Time) and
/// check if it it varies by a certain amount.
///
/// ### Arguments
///
/// * `x_hat` - Solutions to the Least Squares
/// * `coefficient_matrix` - Matrix of linear equations
/// * `constant_matrix` - Matrix of Actual Time
pub fn get_computed_time_and_outliers(
    x_hat: &mut DMatrix<f64>,
    coefficient_matrix: &mut DMatrix<f64>,
    constant_matrix: &mut DMatrix<f64>,
) -> Result<Vec<(usize, f64, f64, f64, bool)>, String> {
    let mut i = 0;
    let mut j = 0;

    // get computed running time
    let mut computed_running_time: Vec<f64> = Vec::new();
    let coeff_row = coefficient_matrix.nrows();
    let coeff_col = coefficient_matrix.ncols();
    while i < coeff_row {
        let mut total_time: f64 = 0.0;
        while j < coeff_col {
            let a_ij = coefficient_matrix[(i, j)];
            total_time += a_ij * x_hat[(j, 0)];
            j += 1;
        }
        computed_running_time.push(total_time);
        i += 1;
        j = 0;
    }

    i = 0;

    // compare w/ margin of error
    let mut outliers: Vec<(usize, f64, f64, f64, bool)> = Vec::new();
    let const_row = constant_matrix.nrows();
    while i < const_row {
        let a_ij = constant_matrix[(i, 0)];

        let numerator = (a_ij - computed_running_time[i]).abs();
        let denominator = computed_running_time[i];
        if approx_eq!(f64, denominator, 0.0, ulps = 2) {
            return Err(String::from("Division by zero"));
        } else {
            let diff = numerator.div(denominator);
            if diff > MARGIN_OF_ERROR {
                // append equation that is an outlier
                outliers.push((i, a_ij, computed_running_time[i], diff, true));
            } else {
                // not outlier, but still display to the user
                // the running time of that Calibration Function
                outliers.push((i, a_ij, computed_running_time[i], diff, false));
            }
        }
        i += 1;
    }

    Ok(outliers)
}

/// Find pivot columns if system of linear eq can't be solved
///
/// ### Arguments
///
/// * `matrix` - An input matrix to solve, typically the RREF'd matrix
fn find_pivot_columns(matrix: &mut DMatrix<f64>) -> Vec<usize> {
    let mut pivot_columns = Vec::new();
    let ncols = matrix.ncols() - 1;

    for j in 0..ncols {
        let mut has_pivot = false;
        for i in 0..matrix.nrows() {
            if !approx_eq!(f64, matrix[(i, j)], 0.0, ulps = 2) {
                has_pivot = true;
                break;
            }
        }
        if has_pivot {
            pivot_columns.push(j);
        }
    }

    pivot_columns
}

/// Reduced row echelon form (RREF) with partial pivoting
///
/// ### Arguments
///
/// * `matrix` - A matrix to perform RREF
fn rref(matrix: &mut DMatrix<f64>) {
    /*let (rows, cols) = matrix.shape();
    let num_pivots = rows.min(cols);

    for pivot_row in 0..num_pivots {
        partial_pivoting(matrix, pivot_row);

        let pivot_val = matrix[(pivot_row, pivot_row)];

        // Scale the pivot row to make the pivot element 1
        for j in pivot_row..cols {
            matrix[(pivot_row, j)] /= pivot_val;
        }

        // Eliminate other rows' entries in the current column
        for i in 0..rows {
            if i != pivot_row {
                let factor = matrix[(i, pivot_row)];
                for j in pivot_row..cols {
                    matrix[(i, j)] -= factor * matrix[(pivot_row, j)];
                }
            }
        }
    }*/

    let (nrows, ncols) = matrix.shape();
    let mut lead = 0;

    for r in 0..nrows {
        if ncols <= lead {
            break;
        }

        let mut i = r;

        while matrix[(i, lead)] == 0.0 {
            i += 1;

            if nrows == i {
                i = r;
                lead += 1;

                if ncols == lead {
                    return;
                }
            }
        }

        if i != r {
            matrix.swap_rows(i, r);
        }

        let pivot = matrix[(r, lead)];

        for j in 0..ncols {
            matrix[(r, j)] /= pivot;
        }

        for i in 0..nrows {
            if i != r {
                let factor = matrix[(i, lead)];
                for j in 0..ncols {
                    matrix[(i, j)] -= factor * matrix[(r, j)];
                }
            }
        }

        lead += 1;
    }
}

/// Perform partial pivoting on the rows to find the maximum pivot element
///
/// ### Arguments
///
/// * `matrix` - Augmented matrix
/// * `pivot_row` - Current row of the matrix
fn partial_pivoting(matrix: &mut DMatrix<f64>, pivot_row: usize) {
    // Find the row with the maximum absolute value in the current column
    let mut max_val = matrix[(pivot_row, pivot_row)].abs();
    let mut max_row = pivot_row;

    for i in pivot_row + 1..matrix.nrows() {
        let val = matrix[(i, pivot_row)].abs();
        if val > max_val {
            max_val = val;
            max_row = i;
        }
    }

    // Swap the current row with the row containing the maximum value
    if max_row != pivot_row {
        matrix.swap_rows(pivot_row, max_row);
    }
}

/// If the matrix is not invertible, we should check if every row
/// has one approximate value of 1. If that is true, then there
/// aren't linear combinations of variables that are dependent
/// of each other, but instead, the entire system is not solvable.
///
/// ### Arguments
///
/// * `matrix` - Augmented matrix that has been RREF'd
fn validate_rref_rows_are_one(matrix: &mut DMatrix<f64>) -> bool {
    let mut i = 0;
    let mut j = 0;

    let mut is_all_one = true;
    while i < matrix.nrows() {
        let mut one_count = 0;

        while j < matrix.ncols() - 1 {
            let a_ij = matrix[(i, j)];
            if approx_eq!(f64, a_ij, 1.0, ulps = 2) {
                one_count += 1;
            }
            j += 1;
        }

        if one_count > 1 {
            is_all_one = false;
        }

        i += 1;
        j = 0;
    }

    is_all_one
}

/// If the matrix is not invertible, and there exists a row that has more than one
/// approximate value of 1, then we should report all linear combinations that exist
/// to the user by scanning each row and only reporting those rows.
///
/// Source: https://stackoverflow.com/questions/43619121/how-to-find-partial-solutions-in-a-underdetermined-system-of-linear-equations
///
/// ### Arguments
///
/// * `matrix` - Augmented matrix that has been RREF'd
fn find_linear_independent_variables(matrix: &mut DMatrix<f64>) -> Vec<usize> {
    let mut i = 0;

    let mut linear_combos = Vec::new();
    while i < matrix.nrows() {
        let mut j = 0;

        let mut max_val: f64 = 0.0;
        let mut k = 0;
        while k < matrix.ncols() - 1 {
            let a_ik = matrix[(i, k)];
            if a_ik > max_val {
                max_val = a_ik;
            }
            k += 1;
        }

        if approx_eq!(f64, max_val, 0.0, ulps = 2) {
            // ignore this row
            i += 1;
            continue;
        }

        let mut independent_vars = Vec::new();
        while j < matrix.ncols() - 1 {
            let a_ij = matrix[(i, j)];
            let ratio = a_ij / max_val;
            // if each element / max_val is approximately not 0
            if !approx_eq!(f64, ratio, 0.0, ulps = 2) {
                independent_vars.push((i, j));
            }

            j += 1;
        }

        if independent_vars.len() == 1 {
            // mark all linear combinations as undetermined
            for (_, gas_param) in independent_vars {
                linear_combos.push(gas_param);
            }
        }

        i += 1;
    }

    linear_combos
}
