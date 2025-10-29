// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Auxiliary function for lagrange interpolation

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
pub fn compute_mult_tree<F: FftField>(roots: &[F]) -> Vec<Vec<DensePolynomial<F>>> {
    // Convert each root `u` into a linear polynomial (x - u)
    let mut bases: Vec<DensePolynomial<F>> = roots
        .into_iter()
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
    for i in 1..=(num_leaves.ilog2() as usize) {
        // Number of nodes at the current level
        let len_at_i = 2usize.pow(depth as u32 - i as u32);

        // Compute the polynomials at this level in parallel:
        let result_at_i = (0..len_at_i)
            .into_par_iter()
            .map(|j| result[i - 1][2 * j].clone() * &result[i - 1][2 * j + 1])
            .collect();
        result.push(result_at_i);
    }

    result
}

/// Given a multiplication tree `mult_tree` (as produced by `compute_mult_tree()`), this function
/// returns the product of all leaf polynomials **except** the one at `divisor_index`.
pub fn quotient<F: FftField>(
    mult_tree: &Vec<Vec<DensePolynomial<F>>>,
    divisor_index: usize,
) -> DensePolynomial<F> {
    // Clone the tree so we can modify it without affecting the original
    let mut mult_tree = mult_tree.clone();

    // Replace the polynomial at the given leaf index with the constant "1"
    mult_tree[0][divisor_index] = DenseUVPolynomial::from_coefficients_vec(vec![F::one()]);

    let depth = mult_tree.len();

    let mut subtree_with_divisor = divisor_index;

    // Recompute the parent nodes along the path to the root
    for i in 1..depth {
        // Move to the parent node index
        subtree_with_divisor /= 2;

        // Recalculate the parent's polynomial as the product of its updated children
        mult_tree[i][subtree_with_divisor] = mult_tree[i - 1][2 * subtree_with_divisor].clone()
            * &mult_tree[i - 1][2 * subtree_with_divisor + 1];
    }

    // The root polynomial now represents the product of all factors except the excluded one
    mult_tree[depth - 1][0].clone()
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

    #[test]
    fn test_quotient() {
        let mut rng = thread_rng();

        for num_roots in 2..=16 {
            let mult_tree = compute_mult_tree(
                &(0..num_roots)
                    .map(|_| Fr::rand(&mut rng))
                    .collect::<Vec<Fr>>(),
            );

            let vanishing_poly = &mult_tree[mult_tree.len() - 1][0];

            for i in 0..num_roots {
                let divisor = &mult_tree[0][i];
                let quotient = quotient(&mult_tree, i);

                assert_eq!(quotient * divisor, *vanishing_poly);
            }
        }
    }
}
