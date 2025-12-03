// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

//! Auxiliary function for Lagrange interpolation

use ark_ff::FftField;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};

/// Recursively computes the **vanishing polynomial** for a given set of points
/// using a divide-and-conquer approach.
///
/// A vanishing polynomial `V(x)` is a polynomial that evaluates to zero at each
/// of the points in `xs`. Formally:
/// ```text
///     V(x_i) = 0  for all x_i in xs
/// ```
pub fn from_roots<F: FftField>(roots: &[F]) -> DensePolynomial<F> {
    match roots.len() {
        0 => DensePolynomial::from_coefficients_vec(vec![F::one()]), // Empty product = 1
        1 => DensePolynomial::from_coefficients_vec(vec![-roots[0], F::one()]), // Single root
        _ => {
            let mid = roots.len() / 2;
            let left = from_roots(&roots[..mid]);
            let right = from_roots(&roots[mid..]);
            &left * &right // This uses FftField
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
            let product = from_roots(&frs);

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
