// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines a trait `DifferentiableFn` for differentiable functions (like polynomials),
//! and provides an implementation for `DensePolynomial` from the arkworks library.

use ark_ff::FftField;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};

/// A trait for functions that can be differentiated. TODO: this is duplicate with aptos-dkg/polynomials
pub trait DifferentiableFn {
    /// Compute the derivative of `self`, returning a new instance of the same type.
    fn differentiate(&self) -> Self;
}

impl<F: FftField> DifferentiableFn for DensePolynomial<F> {
    fn differentiate(&self) -> Self {
        let result_coeffs: Vec<F> = self
            .coeffs()
            .into_iter()
            .skip(1)
            .enumerate()
            .map(|(i, x)| *x * F::from(i as u64 + 1))
            .collect();

        Self::from_coefficients_vec(result_coeffs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_ff::UniformRand;
    use ark_std::{rand::thread_rng, Zero};

    #[test]
    fn test_zero() {
        let p = DensePolynomial::<Fr>::zero();
        let d = p.differentiate();
        assert!(d.coeffs.is_empty()); // derivative of zero polynomial is zero
    }

    #[test]
    fn test_constant() {
        let mut rng = thread_rng();
        let constant = Fr::rand(&mut rng);
        let p = DensePolynomial::from_coefficients_vec(vec![constant]);
        let d = p.differentiate();
        assert!(d.coeffs.is_empty()); // derivative of constant is zero
    }

    #[test]
    fn test_differentiate_linear() {
        let mut rng = thread_rng();
        let a0 = Fr::rand(&mut rng);
        let a1 = Fr::rand(&mut rng);
        let p = DensePolynomial::from_coefficients_vec(vec![a0, a1]);
        let d = p.differentiate();
        assert_eq!(d.coeffs, vec![a1]); // derivative of a0 + a1*x is a1
    }

    #[test]
    fn test_differentiate_quadratic() {
        let mut rng = thread_rng();
        let a0 = Fr::rand(&mut rng);
        let a1 = Fr::rand(&mut rng);
        let a2 = Fr::rand(&mut rng);
        let p = DensePolynomial::from_coefficients_vec(vec![a0, a1, a2]);
        let d = p.differentiate();
        assert_eq!(d.coeffs, vec![a1, a2 + a2]); // derivative of a0 + a1*x + a2*x² is a1 + 2*a2*x
    }
}
