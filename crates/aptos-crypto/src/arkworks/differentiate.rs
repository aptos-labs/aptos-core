// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines a trait `DifferentiableFn` for differentiable functions (like polynomials),
//! and provides an implementation for `DensePolynomial` from the arkworks library.

use ark_ff::Field;
use ark_poly::univariate::DensePolynomial;

/// A trait for functions that can be differentiated.
pub trait DifferentiableFn {
    /// Differentiate `self` in place
    fn differentiate_in_place(&mut self);

    /// Return a new differentiated instance
    fn differentiate(&self) -> Self
    where
        Self: Clone,
    {
        let mut copy = self.clone();
        copy.differentiate_in_place();
        copy
    }
}

impl<F: Field> DifferentiableFn for DensePolynomial<F> {
    fn differentiate_in_place(&mut self) {
        if self.coeffs.len() <= 1 {
            // Zero or constant polynomial
            self.coeffs.clear();
            return;
        }

        for i in 1..self.coeffs.len() {
            self.coeffs[i - 1] = self.coeffs[i] * F::from(i as u64);
        }
        self.coeffs.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_ff::UniformRand;
    use ark_poly::DenseUVPolynomial;
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
