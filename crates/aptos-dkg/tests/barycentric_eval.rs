// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use aptos_dkg::algebra::polynomials::barycentric_eval;
use ark_ec::{pairing::Pairing, AdditiveGroup};
use ark_ff::{FftField, Field, UniformRand};
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial, Polynomial};
use ark_std::{rand::Rng, test_rng};

#[cfg(test)]
fn run_barycentric_case<E: Pairing>(degree: usize, n: usize, sample_points: usize) {
    let mut rng = test_rng();
    type Fr<E> = <E as Pairing>::ScalarField;

    // Generate coefficients for a random polynomial; if degree is 0, randomly sometimes replace with the zero polynomial
    let poly_coeffs: Vec<Fr<E>> = if degree == 0 && rng.gen_bool(0.5) {
        vec![Fr::<E>::ZERO]
    } else {
        (0..=degree).map(|_| Fr::<E>::rand(&mut rng)).collect()
    };
    let poly = DensePolynomial::from_coefficients_vec(poly_coeffs);

    // Build the interpolation domain and evaluations
    let omega = Fr::<E>::get_root_of_unity(n as u64)
        .unwrap_or_else(|| panic!("No root of unity of size {} for this field", n));
    let omegas: Vec<Fr<E>> = (0..n).map(|i| omega.pow([i as u64])).collect();

    // Evaluate the polynomial at each root of unity
    let evals: Vec<Fr<E>> = omegas.iter().map(|&r| poly.evaluate(&r)).collect();

    // Precompute the multiplicative inverse of n for barycentric evaluation
    let n_inv = Fr::<E>::from(n as u64).inverse().unwrap();

    // Test barycentric interpolation at random points
    for _ in 0..sample_points {
        let x = Fr::<E>::rand(&mut rng);
        let expected = poly.evaluate(&x);
        let val = barycentric_eval(&evals, &omegas, x, n_inv);
        assert_eq!(
            val, expected,
            "Failed for degree {}, n = {} at x = {:?}",
            degree, n, x
        );
    }

    // Test barycentric interpolation at the roots themselves, so check that the function returns the known values at the domain points
    for (omega, &eval) in omegas.iter().zip(evals.iter()) {
        let val = barycentric_eval(&evals, &omegas, *omega, n_inv);
        assert_eq!(
            val, eval,
            "Interpolation mismatch at root {:?} for degree {}, domain size n = {}",
            omega, degree, n
        );
    }
}

#[test]
fn test_barycentric_eval() {
    use ark_bls12_381::Bls12_381;
    use ark_bn254::Bn254;
    let cases = [
        (0, 1),
        (0, 2),
        (1, 2),
        (0, 4),
        (1, 4),
        (2, 4),
        (3, 4),
        (0, 8),
        (1, 8),
        (4, 8),
        (7, 8),
        (0, 16),
        (1, 16),
        (8, 16),
        (15, 16),
        (0, 32),
        (1, 32),
        (16, 32),
        (31, 32),
    ];

    for &(degree, n) in &cases {
        run_barycentric_case::<Bn254>(degree, n, 5);
    }
    for &(degree, n) in &cases {
        run_barycentric_case::<Bls12_381>(degree, n, 5);
    }
}
