// Copyright © Velor Foundation
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
    for j in 0..ncols {
        coefficient_matrix[(idx, j)] = formula[j];
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
    let nrows = augmented_matrix.nrows();
    let ncols = coefficient_matrix.ncols();
    for i in 0..nrows {
        for j in 0..ncols {
            augmented_matrix[(i, j)] = coefficient_matrix[(i, j)];
        }
    }

    for i in 0..nrows {
        augmented_matrix[(i, ncols)] = constant_matrix[(i, 0)];
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
    // get computed running time
    let mut computed_running_time: Vec<f64> = Vec::new();
    let coeff_row = coefficient_matrix.nrows();
    let coeff_col = coefficient_matrix.ncols();
    for i in 0..coeff_row {
        let mut total_time: f64 = 0.0;
        for j in 0..coeff_col {
            let a_ij = coefficient_matrix[(i, j)];
            total_time += a_ij * x_hat[(j, 0)];
        }
        computed_running_time.push(total_time);
    }

    // compare w/ margin of error
    let mut outliers: Vec<(usize, f64, f64, f64, bool)> = Vec::new();
    let const_row = constant_matrix.nrows();
    for i in 0..const_row {
        let a_ij = constant_matrix[(i, 0)];

        let numerator = (a_ij - computed_running_time[i]).abs();
        let denominator = computed_running_time[i];
        if approx_eq!(f64, denominator, 0.0, ulps = 2) {
            return Err(String::from("Division by zero"));
        } else {
            let diff = numerator.div(denominator);
            if diff > MARGIN_OF_ERROR {
                // append equation that is an outlier
                outliers.push((i, computed_running_time[i], a_ij, diff, true));
            } else {
                // not outlier, but still display to the user
                // the running time of that Calibration Function
                outliers.push((i, computed_running_time[i], a_ij, diff, false));
            }
        }
    }

    Ok(outliers)
}

/// Reduced row echelon form (RREF) with partial pivoting
///
/// ### Arguments
///
/// * `matrix` - A matrix to perform RREF
fn rref(matrix: &mut DMatrix<f64>) {
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
    let mut linear_combos = Vec::new();

    for i in 0..matrix.nrows() {
        let mut max_val: f64 = 0.0;
        for k in 0..(matrix.ncols() - 1) {
            let a_ik = matrix[(i, k)];
            if a_ik > max_val {
                max_val = a_ik;
            }
        }

        if approx_eq!(f64, max_val, 0.0, ulps = 2) {
            // ignore this row
            continue;
        }

        let mut independent_vars = Vec::new();
        for j in 0..(matrix.ncols() - 1) {
            let a_ij = matrix[(i, j)];
            let ratio = a_ij / max_val;
            // if each element / max_val is approximately not 0
            if !approx_eq!(f64, ratio, 0.0, ulps = 2) {
                independent_vars.push((i, j));
            }
        }

        if independent_vars.len() == 1 {
            // mark all linear combinations as undetermined
            for (_, gas_param) in independent_vars {
                linear_combos.push(gas_param);
            }
        }
    }

    linear_combos
}
