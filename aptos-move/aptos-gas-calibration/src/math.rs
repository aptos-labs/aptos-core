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
        j = j + 1;
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
            j = j + 1;
        }
        i = i + 1;
        j = 0;
    }

    i = 0;
    while i < nrows {
        augmented_matrix[(i, ncols)] = constant_matrix[(i, 0)];
        i = i + 1;
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

/// Find all free variables which is the pivot columns
///
/// ### Arguments
///
/// * `A` - Coefficient matrix
/// * `b` - Constant matrix
#[allow(non_snake_case)]
pub fn find_free_variables(A: &mut DMatrix<f64>, b: &mut DMatrix<f64>) -> Vec<usize> {
    let A_T = A.transpose();
    let mut A_TA = A_T.clone().mul(A.clone());
    let mut A_Tb = A_T.clone().mul(b.clone());

    let nrows_a_ta = A_TA.nrows();
    let ncols_a_ta = A_TA.ncols();
    let mut aug_matrix = DMatrix::<f64>::zeros(nrows_a_ta, ncols_a_ta + 1);
    create_augmented_matrix(&mut aug_matrix, &mut A_TA, &mut A_Tb);
    rref(&mut aug_matrix);

    let pivot_columns = find_pivot_columns(&mut aug_matrix);
    pivot_columns
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
pub fn find_outliers(
    x_hat: &mut DMatrix<f64>,
    coefficient_matrix: &mut DMatrix<f64>,
    constant_matrix: &mut DMatrix<f64>,
) -> Result<Vec<(usize, f64, f64, f64)>, String> {
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
            total_time = total_time + (a_ij * x_hat[(j, 0)]);
            j = j + 1;
        }
        computed_running_time.push(total_time);
        i = i + 1;
        j = 0;
    }

    i = 0;

    // compare w/ margin of error
    let mut outliers: Vec<(usize, f64, f64, f64)> = Vec::new();
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
                outliers.push((i, a_ij, computed_running_time[i], diff));
            }
        }
        i = i + 1;
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

/// Reduced row echelon form (RREF)
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

        while approx_eq!(f64, matrix[(i, lead)], 0.0, ulps = 2) {
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
