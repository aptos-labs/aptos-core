// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Auxiliary function for Lagrange interpolation

use ark_ff::FftField;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};

/// Vanishing polynomial for a bunch of points
pub fn vanishing_poly<F: FftField>(xs: &[F]) -> DensePolynomial<F> {
    compute_product(xs)
}

/// Recursively computes the product polynomial of all `(x - root)`
/// using a divide-and-conquer approach.
fn compute_product<F: FftField>(roots: &[F]) -> DensePolynomial<F> {
    match roots.len() {
        0 => DensePolynomial::from_coefficients_vec(vec![F::one()]), // Empty product = 1
        1 => DensePolynomial::from_coefficients_vec(vec![-roots[0], F::one()]), // Single root
        _ => {
            let mid = roots.len() / 2;
            let left = compute_product(&roots[..mid]);
            let right = compute_product(&roots[mid..]);
            &left * &right
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
    use ark_std::{rand::thread_rng, One, UniformRand};

    #[test]
    fn test_compute_product() {
        let mut rng = thread_rng();

        for num_roots in 1..=16 {
            let frs: Vec<Fr> = (0..num_roots).map(|_| Fr::rand(&mut rng)).collect();

            // Compute product using recursive function
            let product = compute_product(&frs);

            // Naive computation of product
            let expected: DensePolynomial<Fr> = frs
                .into_iter()
                .map(|u| DensePolynomial::from_coefficients_vec(vec![-u, Fr::one()]))
                .reduce(|acc, f| acc * f)
                .unwrap();

            assert_eq!(product, expected);
        }
    }
}
