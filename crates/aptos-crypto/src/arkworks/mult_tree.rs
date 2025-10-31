// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Auxiliary function for Lagrange interpolation

use ark_ff::FftField;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};

/// Vanishing polynomial for a bunch of points
pub fn vanishing_poly<F: FftField>(xs: &[F]) -> DensePolynomial<F> {
    compute_mult_tree(xs).last().unwrap()[0].clone()
}

/// This function constructs a binary tree of polynomials where:
/// - The leaves are linear polynomials of the form `(x - root)` for each root in `roots`.
/// - Internal nodes represent the product of their two child polynomials.
///
/// FftField is used because that's needed to multiply a pair of `DenseUVPolynomial` in arkworks
fn compute_mult_tree<F: FftField>(roots: &[F]) -> Vec<Vec<DensePolynomial<F>>> {
    // Convert each root `u` into a linear polynomial (x - u)
    let mut bases: Vec<DensePolynomial<F>> = roots
        .iter()
        .cloned()
        .map(|u| DenseUVPolynomial::from_coefficients_vec(vec![-u, F::one()]))
        .collect();

    // Pad to the next power of two with constant polynomial "1"
    bases.resize(
        bases.len().next_power_of_two(),
        DenseUVPolynomial::from_coefficients_vec(vec![F::one()]),
    );

    let num_leaves = bases.len();
    let mut result = vec![bases];
    let depth = num_leaves.ilog2();
    debug_assert_eq!(2usize.pow(depth), num_leaves);

    // Iteratively build upper levels of the tree
    for i in 1..=(depth as usize) {
        // Number of nodes at the current level
        let len_at_i = 2usize.pow(depth - (i as u32));

        // Compute the polynomials at this level in parallel:
        let result_at_i = (0..len_at_i)
            .into_par_iter()
            .map(|j| result[i - 1][2 * j].clone() * &result[i - 1][2 * j + 1])
            .collect();
        result.push(result_at_i);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
    use ark_std::{rand::thread_rng, One, UniformRand};

    #[test]
    fn test_mult_tree() {
        let mut rng = thread_rng();

        for num_roots in 1..=16 {
            let frs: Vec<Fr> = (0..num_roots).map(|_| Fr::rand(&mut rng)).collect();
            let mult_tree = compute_mult_tree(&frs);

            // Naive computation of root of tree
            let result: DensePolynomial<Fr> = frs
                .into_iter()
                .map(|u| DenseUVPolynomial::from_coefficients_vec(vec![-u, Fr::one()]))
                .reduce(|acc, f| acc * f)
                .unwrap();

            assert_eq!(result, mult_tree.into_iter().last().unwrap()[0]);
        }
    }
}
