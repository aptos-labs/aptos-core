// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Univariate polynomial and compressed form (from Jolt, no jolt dep).

use crate::arkworks::sumcheck::{field::SumcheckField, gaussian_elimination};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
#[derive(Clone, Debug, PartialEq)]
pub struct UniPoly<F: SumcheckField> {
    pub coeffs: Vec<F>,
}

/// Univariate polynomial with the linear coefficient omitted (proof compression).
///
/// Let H(x) = c_0 + c_1·x + c_2·x² + … . In sumcheck, the verifier learns the "hint"
/// e = H(0) + H(1) (the running claim for that round). So:
///
///   e = H(0) + H(1) = c_0 + (c_0 + c_1 + c_2 + …) = 2·c_0 + c_1 + c_2 + c_3 + … .
///
/// The prover sends [c_0, c_2, c_3, …] (all coefficients except c_1). The verifier recovers
///
///   c_1 = e − 2·c_0 − c_2 − c_3 − … ,
///
/// then evaluates H(r) = c_0 + c_1·r + c_2·r² + … at the challenge r. One field element
/// per round is saved by not sending c_1.
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct CompressedUniPoly<F: SumcheckField> {
    /// Constant term, then x^2, x^3, ... (linear coefficient recovered from hint).
    pub coeffs_except_linear_term: Vec<F>,
}

impl<F: SumcheckField> UniPoly<F> {
    pub fn from_coeff(coeffs: Vec<F>) -> Self {
        Self { coeffs }
    }

    pub fn from_evals(evals: &[F]) -> Self {
        let coeffs = vandermonde_interpolation::<F>(evals);
        Self { coeffs }
    }

    pub fn degree(&self) -> usize {
        self.coeffs.len().saturating_sub(1)
    }

    pub fn zero() -> Self {
        Self::from_coeff(Vec::new())
    }

    pub fn evaluate(&self, r: &F::Challenge) -> F {
        let r_f = F::challenge_to_field(r);
        let mut eval = self.coeffs[0];
        let mut power = r_f;
        for coeff in self.coeffs.iter().skip(1) {
            eval += power * coeff;
            power *= r_f;
        }
        eval
    }

    /// Compress: drop linear coefficient (verifier recovers from H(0)+H(1)=hint).
    pub fn compress(&self) -> CompressedUniPoly<F> {
        let coeffs_except_linear_term = if self.coeffs.is_empty() {
            vec![]
        } else if self.coeffs.len() == 1 {
            vec![self.coeffs[0]]
        } else {
            [&self.coeffs[..1], &self.coeffs[2..]].concat()
        };
        CompressedUniPoly {
            coeffs_except_linear_term,
        }
    }
}

impl<F: SumcheckField> std::ops::AddAssign<&UniPoly<F>> for UniPoly<F> {
    fn add_assign(&mut self, rhs: &UniPoly<F>) {
        if rhs.coeffs.len() > self.coeffs.len() {
            self.coeffs.resize(rhs.coeffs.len(), F::zero());
        }
        for (a, b) in self.coeffs.iter_mut().zip(rhs.coeffs.iter()) {
            *a += *b;
        }
    }
}

impl<F: SumcheckField> std::ops::Mul<F> for &UniPoly<F> {
    type Output = UniPoly<F>;

    fn mul(self, rhs: F) -> UniPoly<F> {
        UniPoly::from_coeff(self.coeffs.iter().map(|c| *c * rhs).collect())
    }
}

fn vandermonde_interpolation<F: SumcheckField>(evals: &[F]) -> Vec<F> {
    let n = evals.len();
    let xs: Vec<F> = (0..n).map(|i| F::from(i as u64)).collect();
    let mut vandermonde: Vec<Vec<F>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(n + 1);
        let x = xs[i];
        row.push(F::one());
        row.push(x);
        for _ in 2..n {
            row.push(*row.last().unwrap() * x);
        }
        row.push(evals[i]);
        vandermonde.push(row);
    }
    gaussian_elimination::gaussian_elimination(&mut vandermonde)
}

impl<F: SumcheckField> CompressedUniPoly<F> {
    /// Recover linear term from hint = H(0)+H(1), then evaluate at x.
    pub fn eval_from_hint(&self, hint: &F, x: &F::Challenge) -> F {
        let x_f = F::challenge_to_field(x);
        if self.coeffs_except_linear_term.is_empty() {
            return *hint;
        }
        let mut linear_term =
            *hint - self.coeffs_except_linear_term[0] - self.coeffs_except_linear_term[0];
        for i in 1..self.coeffs_except_linear_term.len() {
            linear_term -= self.coeffs_except_linear_term[i];
        }
        // coeffs_except_linear_term = [c_0, c_2, c_3, ...]; match Jolt: running_point = x then x^2, x^3, ...
        let mut running_point = x_f;
        let mut running_sum = self.coeffs_except_linear_term[0] + x_f * linear_term;
        for i in 1..self.coeffs_except_linear_term.len() {
            running_point *= x_f;
            running_sum += self.coeffs_except_linear_term[i] * running_point;
        }
        running_sum
    }

    pub fn degree(&self) -> usize {
        self.coeffs_except_linear_term.len()
    }
}
