// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
//! Masking polynomial with no mixed terms: g(X_1,...,X_m) = c + sum_{i=0}^{m-1} g_i(X_i).
//! Per [Lib19], round messages and hypercube sum are computable in O(m*d) steps.

use crate::arkworks::sumcheck::{field::SumcheckField, unipoly::UniPoly};
use sha3::{Digest, Keccak256};
use std::marker::PhantomData;

/// Multivariate polynomial with no mixed terms: g = constant + sum_i g_i(X_i).
/// Each g_i is a univariate polynomial of degree at most `degree`.
#[derive(Clone, Debug)]
pub struct MaskingPolynomial<F: SumcheckField> {
    /// Constant term (optional efficiency: can be 0 and fold into g_0).
    pub constant_term: F,
    /// Univariate polynomials g_i(X_i), one per variable. Length = num_vars.
    pub univariate_polys: Vec<UniPoly<F>>,
    _marker: PhantomData<F>,
}

impl<F: SumcheckField> MaskingPolynomial<F> {
    /// Build from explicit constant and list of univariate polynomials (one per variable).
    pub fn new(constant_term: F, univariate_polys: Vec<UniPoly<F>>) -> Self {
        Self {
            constant_term,
            univariate_polys,
            _marker: PhantomData,
        }
    }

    pub fn num_vars(&self) -> usize {
        self.univariate_polys.len()
    }

    pub fn degree(&self) -> usize {
        self.univariate_polys
            .iter()
            .map(|p| p.degree())
            .max()
            .unwrap_or(0)
    }

    /// Hypercube sum H_g = sum_{b in {0,1}^m} g(b) in O(m*d).
    /// H_g = constant_term * 2^m + sum_i 2^{m-1} * (g_i(0) + g_i(1)).
    pub fn hypercube_sum(&self) -> F {
        let m = self.num_vars();
        if m == 0 {
            return self.constant_term;
        }
        let two_m = F::one().mul_pow_2(m); // 2^m
        let two_m_minus_1 = F::one().mul_pow_2(m - 1); // 2^{m-1}
        let mut sum = self.constant_term * two_m;
        for g_i in &self.univariate_polys {
            let g_i_0 = g_i.coeffs.first().copied().unwrap_or(F::zero());
            let g_i_1 = g_i.evaluate(&F::into_challenge(F::one()));
            sum += two_m_minus_1 * (g_i_0 + g_i_1);
        }
        sum
    }

    /// Round-j univariate (0-indexed). P_j(X) = sum_{b in {0,1}^{m-j-1}} g(r_0,...,r_{j-1}, X, b).
    /// There are 2^{m-j-1} terms (variable j is X, so we sum over the remaining m-j-1 variables).
    /// So P_j(X) = 2^{m-j-1}*(constant_term + sum_{i=0}^{j-1} g_i(r_i) + g_j(X)) + sum_{i=j+1}^{m-1} 2^{m-j-2}(g_i(0)+g_i(1)).
    /// Computed in O(m*d). `r_prev` = [r_0, ..., r_{j-1}] (challenges from previous rounds).
    pub fn round_message(&self, j: usize, r_prev: &[F::Challenge]) -> UniPoly<F> {
        let m = self.num_vars();
        assert!(j < m, "round index j must be < num_vars");
        assert_eq!(r_prev.len(), j, "r_prev must have length j");

        // scale = 2^{m-j-1} (number of summands for the (c + prefix + g_j(X)) part)
        let scale_exp = m - j - 1;
        let scale = if scale_exp == 0 {
            F::one()
        } else {
            F::one().mul_pow_2(scale_exp)
        };

        // scale * g_j(X)
        let g_j = &self.univariate_polys[j];
        let mut h_j = UniPoly::from_coeff(g_j.coeffs.iter().map(|c| *c * scale).collect());

        // + scale * (constant_term + sum_{i=0}^{j-1} g_i(r_i))
        let mut prefix_sum = self.constant_term;
        for (i, r_i) in r_prev.iter().enumerate() {
            prefix_sum += self.univariate_polys[i].evaluate(r_i);
        }
        h_j.coeffs[0] += scale * prefix_sum;

        // + sum_{i=j+1}^{m-1} 2^{m-j-2} (g_i(0) + g_i(1))
        for i in (j + 1)..m {
            let factor_exp = m - j - 2; // 2^{m-j-2} for each remaining variable
            let factor = if factor_exp == 0 {
                F::one()
            } else {
                F::one().mul_pow_2(factor_exp)
            };
            let g_i_0 = self.univariate_polys[i]
                .coeffs
                .first()
                .copied()
                .unwrap_or(F::zero());
            let g_i_1 = self.univariate_polys[i].evaluate(&F::into_challenge(F::one()));
            h_j.coeffs[0] += factor * (g_i_0 + g_i_1);
        }

        h_j
    }

    /// Evaluate g at point r = (r_0, ..., r_{m-1}). O(m*d).
    pub fn evaluate(&self, r: &[F::Challenge]) -> F {
        assert_eq!(r.len(), self.num_vars());
        let mut out = self.constant_term;
        for (g_i, r_i) in self.univariate_polys.iter().zip(r.iter()) {
            out += g_i.evaluate(r_i);
        }
        out
    }
}

/// Expand seed bytes into field elements via Keccak256. Requires PrimeField for from_be_bytes_mod_order.
pub fn expand_seed_to_field<F: SumcheckField + ark_ff::PrimeField>(
    seed: &[u8],
    count: usize,
) -> Vec<F> {
    let mut out = Vec::with_capacity(count);
    let mut state = Vec::from(seed);
    for _ in 0..count {
        let mut hasher = Keccak256::new();
        hasher.update(&state);
        state = hasher.finalize().to_vec();
        let mut buf = [0u8; 32];
        let take = state.len().min(32);
        buf[..take].copy_from_slice(&state[..take]);
        out.push(F::from_be_bytes_mod_order(&buf));
    }
    out
}

impl<F: SumcheckField + ark_ff::PrimeField> MaskingPolynomial<F> {
    /// Build a random masking polynomial of given num_vars and degree from a seed (e.g. from transcript).
    /// Uses constant_term = first expansion, then m univariate polynomials of degree `degree` (each degree+1 coefficients).
    /// Total coefficients: 1 + m * (degree + 1).
    pub fn from_seed(seed: &[u8], num_vars: usize, degree: usize) -> Self {
        let total_coeffs = 1 + num_vars * (degree + 1);
        let coeffs = expand_seed_to_field::<F>(seed, total_coeffs);
        let constant_term = coeffs[0];
        let mut univariate_polys = Vec::with_capacity(num_vars);
        for i in 0..num_vars {
            let start = 1 + i * (degree + 1);
            let end = start + degree + 1;
            let poly_coeffs = coeffs[start..end].to_vec();
            univariate_polys.push(UniPoly::from_coeff(poly_coeffs));
        }
        Self::new(constant_term, univariate_polys)
    }
}
