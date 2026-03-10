// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Inspired by: https://github.com/TheAlgorithms/Rust/blob/master/src/math/gaussian_elimination.rs

use crate::arkworks::sumcheck::field::SumcheckField;
pub fn gaussian_elimination<F: SumcheckField>(matrix: &mut [Vec<F>]) -> Vec<F> {
    let size = matrix.len();
    assert_eq!(size, matrix[0].len() - 1);

    for i in 0..size - 1 {
        for j in i..size - 1 {
            echelon(matrix, i, j);
        }
    }

    for i in (1..size).rev() {
        eliminate(matrix, i);
    }

    let mut result: Vec<F> = vec![F::zero(); size];
    for i in 0..size {
        result[i] = matrix[i][size] / matrix[i][i];
    }
    result
}

fn echelon<F: SumcheckField>(matrix: &mut [Vec<F>], i: usize, j: usize) {
    let size = matrix.len();
    if matrix[i][i] != F::zero() {
        let factor = matrix[j + 1][i] / matrix[i][i];
        for k in i..size + 1 {
            let tmp = matrix[i][k];
            matrix[j + 1][k] -= factor * tmp;
        }
    }
}

fn eliminate<F: SumcheckField>(matrix: &mut [Vec<F>], i: usize) {
    let size = matrix.len();
    if matrix[i][i] != F::zero() {
        for j in (1..=i).rev() {
            let factor = matrix[j - 1][i] / matrix[i][i];
            for k in (0..=size).rev() {
                let tmp = matrix[i][k];
                matrix[j - 1][k] -= factor * tmp;
            }
        }
    }
}
