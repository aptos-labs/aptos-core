// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Auxiliary function for Lagrange interpolation

use ark_ff::FftField;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
use ark_ff::Field;

const FFT_THRESH: usize = 64 * 16; // Given that our `n` is small in practice, we can increase this further, doesn't matter

/// Recursively computes the **vanishing polynomial** for a given set of points
/// using a divide-and-conquer approach.
///
/// A vanishing polynomial `V(x)` is a polynomial that evaluates to zero at each
/// of the points in `xs`. Formally:
/// ```text
///     V(x_i) = 0  for all x_i in xs
/// ```
/// In other words, V(X) = \prod_{x_i in xs} (X - x_i)
pub fn from_roots<F: FftField>(roots: &[F]) -> DensePolynomial<F> {
    match roots.len() {
        0 => DensePolynomial::from_coefficients_vec(vec![F::one()]), // Is this correct? F::one() or empty vec?
        1 => DensePolynomial::from_coefficients_vec(vec![-roots[0], F::one()]),
        2 => {
            let (a, b) = (roots[0], roots[1]);
            DensePolynomial::from_coefficients_vec(vec![
                a * b,
                -(a + b),
                F::one(),
            ])
        }
        3 => {
            let (a, b, c) = (roots[0], roots[1], roots[2]);
            DensePolynomial::from_coefficients_vec(vec![
                -(a * b * c),
                a * b + a * c + b * c,
                -(a + b + c),
                F::one(),
            ])
        } // Not sure 2 and 3 are really useful
        _ => {
            let mid = roots.len() / 2;
            let (left, right) = rayon::join(
                || from_roots(&roots[..mid]),
                || from_roots(&roots[mid..]),
            );

            let result_len = left.coeffs.len() + right.coeffs.len() - 1;
            let dom_size = result_len.next_power_of_two();

            if dom_size < FFT_THRESH {
                naive_poly_mul(&left, &right)
            } else {
                &left * &right
            }
        }
    }
}

fn naive_poly_mul<F: Field>(
    a: &DensePolynomial<F>,
    b: &DensePolynomial<F>,
) -> DensePolynomial<F> {
    let a_coeffs = &a.coeffs;
    let b_coeffs = &b.coeffs;

    let mut out = vec![F::zero(); a_coeffs.len() + b_coeffs.len() - 1];

    for (i, ai) in a_coeffs.iter().enumerate() {
        for (j, bj) in b_coeffs.iter().enumerate() {
            out[i + j] += *ai * *bj;
        }
    }

    DensePolynomial::from_coefficients_vec(out)
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
