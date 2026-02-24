//! Prover
use crate::sumcheck::ml_sumcheck::{
    data_structures::BinaryConstraintPolynomial,
    protocol::{verifier::VerifierMsg, IPForMLSumcheck},
};
use ark_ff::Field;
use ark_poly::{DenseMultilinearExtension, MultilinearExtension};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{cfg_iter_mut, vec::Vec};
#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "range_proof_timing_multivariate")]
use std::time::Instant;

/// Prover Message
#[derive(Clone, CanonicalSerialize, CanonicalDeserialize)]
pub struct ProverMsg<F: Field> {
    /// evaluations on P(0), P(1), P(2), ...
    pub(crate) evaluations: Vec<F>,
}

/// Prover State for binary constraints with eq_t masking and g polynomial
pub struct ProverState<F: Field> {
    /// sampled randomness given by the verifier
    pub randomness: Vec<F>,
    /// Optional linear term (fixed each round)
    pub linear_term: Option<DenseMultilinearExtension<F>>,
    /// List of (coefficient, polynomial) pairs
    pub constraints: Vec<(F, DenseMultilinearExtension<F>)>,
    /// The eq_t point (original, never modified)
    pub eq_point_original: Vec<F>,
    /// Coefficient α for g term
    pub alpha: F,
    /// Random univariate polynomials g₁, ..., gₙ (coefficients)
    pub g_polys: Vec<Vec<F>>,
    /// Number of variables
    pub num_vars: usize,
    /// The current round number
    pub round: usize,
}

impl<F: Field> IPForMLSumcheck<F> {
    /// Initialize the prover for binary constraint polynomial with eq masking and g
    pub fn prover_init(polynomial: &BinaryConstraintPolynomial<F>) -> ProverState<F> {
        if polynomial.num_variables == 0 {
            panic!("Attempt to prove a constant.");
        }

        // Clone all polynomials
        let constraints = polynomial
            .constraints
            .iter()
            .map(|(c, p)| (*c, p.clone()))
            .collect();

        ProverState {
            randomness: Vec::with_capacity(polynomial.num_variables),
            linear_term: polynomial.linear_term.clone(),
            constraints,
            eq_point_original: polynomial.eq_point.clone(),
            alpha: polynomial.alpha,
            g_polys: polynomial.g_polys.clone(),
            num_vars: polynomial.num_variables,
            round: 0,
        }
    }

    /// Receive message from verifier, generate prover message, and proceed to next round
    #[allow(unused_variables)]
    pub fn prove_round(
        prover_state: &mut ProverState<F>,
        v_msg: &Option<VerifierMsg<F>>,
        timing: &mut Option<&mut dyn FnMut(&str, std::time::Duration)>,
    ) -> ProverMsg<F> {
        #[cfg(feature = "range_proof_timing_multivariate")]
        let start_fix = Instant::now();
        if let Some(msg) = v_msg {
            if prover_state.round == 0 {
                panic!("first round should be prover first.");
            }
            prover_state.randomness.push(msg.randomness);

            // Fix variables in all polynomials
            let r = prover_state.randomness[prover_state.round - 1];
            if let Some(ref mut linear) = prover_state.linear_term {
                *linear = linear.fix_variables(&[r]);
            }
            cfg_iter_mut!(prover_state.constraints).for_each(|(_, poly)| {
                *poly = poly.fix_variables(&[r]);
            });
        } else if prover_state.round > 0 {
            panic!("verifier message is empty");
        }
        #[cfg(feature = "range_proof_timing_multivariate")]
        if let Some(f) = timing {
            let round_idx = prover_state.round; // 0-based round we're about to compute
            f(
                &format!("sumcheck round {} fix_variables", round_idx),
                start_fix.elapsed(),
            );
        }

        prover_state.round += 1;

        if prover_state.round > prover_state.num_vars {
            panic!("Prover is not active");
        }

        let i = prover_state.round;
        let nv = prover_state.num_vars;
        #[cfg(feature = "range_proof_timing_multivariate")]
        let round_idx = i - 1;

        // Degree is 4
        let degree = 4;

        // eq_t = ∏_j [ x_j·t_j + (1-x_j)(1-t_j) ]. For remaining vars only the b that matches
        // the last (nv-i) coordinates of t gives non-zero product. So we use that single b only.
        let one = F::one();
        let two = F::from(2u64);
        let mut b_star = 0usize;
        for j in 0..(nv - i) {
            if !prover_state.eq_point_original[i + j].is_zero() {
                b_star |= 1 << j;
            }
        }
        let b = b_star;

        #[cfg(feature = "range_proof_timing_multivariate")]
        let start_fold = Instant::now();

        let mut products_sum = vec![F::zero(); degree + 1];

        // Single b that matches the last (nv-i) coordinates of t; remaining eq_t factors are 1.
        let linear_contrib = if let Some(ref linear) = prover_state.linear_term {
            let l0 = linear[b << 1];
            let l1 = linear[(b << 1) + 1];
            (l0, l1)
        } else {
            (F::zero(), F::zero())
        };

        for x in 0..=degree {
            let x_field = F::from(x as u64);

            // eq_t: fixed vars + current var; remaining vars contribute 1 (b matches t)
            let mut eq_val = one;
            for j in 0..i - 1 {
                let tj = prover_state.eq_point_original[j];
                let rj = prover_state.randomness[j];
                eq_val *= (one - tj) + rj * (two * tj - one);
            }
            let ti = prover_state.eq_point_original[i - 1];
            eq_val *= (one - ti) + x_field * (two * ti - one);

            // eq_{0,...,0}(x) = ∏ᵢ(1-xᵢ)
            let mut eq_zero_val = one;
            for j in 0..i - 1 {
                eq_zero_val *= one - prover_state.randomness[j];
            }
            eq_zero_val *= one - x_field;
            for j in 0..(nv - i) {
                let xj = if (b >> j) & 1 == 1 { one } else { F::zero() };
                eq_zero_val *= one - xj;
            }

            let linear_val = linear_contrib.0 + x_field * (linear_contrib.1 - linear_contrib.0);
            products_sum[x] += linear_val * eq_val * (one - eq_zero_val);
        }

        for (coefficient, poly) in &prover_state.constraints {
            let p0 = poly[b << 1];
            let p1 = poly[(b << 1) + 1];
            let delta = p1 - p0;
            let a0 = p0 * (one - p0);
            let a1 = delta * (one - two * p0);
            let a2 = -(delta * delta);

            for x in 0..=degree {
                let x_field = F::from(x as u64);

                let mut eq_val = one;
                for j in 0..i - 1 {
                    let tj = prover_state.eq_point_original[j];
                    let rj = prover_state.randomness[j];
                    eq_val *= (one - tj) + rj * (two * tj - one);
                }
                let ti = prover_state.eq_point_original[i - 1];
                eq_val *= (one - ti) + x_field * (two * ti - one);

                let mut eq_zero_val = one;
                for j in 0..i - 1 {
                    eq_zero_val *= one - prover_state.randomness[j];
                }
                eq_zero_val *= one - x_field;
                for j in 0..(nv - i) {
                    let xj = if (b >> j) & 1 == 1 { one } else { F::zero() };
                    eq_zero_val *= one - xj;
                }
                let binary_val = a0 + a1 * x_field + a2 * x_field * x_field;
                products_sum[x] += *coefficient * binary_val * eq_val * (one - eq_zero_val);
            }
        }
        #[cfg(feature = "range_proof_timing_multivariate")]
        if let Some(f) = timing {
            f(
                &format!("sumcheck round {} fold (hypercube sum)", round_idx),
                start_fold.elapsed(),
            );
        }

        // Add α·g terms
        #[cfg(feature = "range_proof_timing_multivariate")]
        let start_g = Instant::now();

        // Contribution from fixed variables (constant term)
        // Contribution from fixed variables (constant term)
        let mut fixed_g_sum = F::zero();
        for j in 0..(i - 1) {
            let rj = prover_state.randomness[j];
            let coeffs = &prover_state.g_polys[j];
            let mut g_j_val = coeffs[0];
            let mut rj_pow = rj;
            for k in 1..5 {
                g_j_val += coeffs[k] * rj_pow;
                rj_pow *= rj;
            }
            fixed_g_sum += g_j_val;
        }

        if i > 1 {
            let num_all_remaining = F::from(1u64 << (nv - i)); // Changed from (nv - i + 1)
            let fixed_contribution = prover_state.alpha * fixed_g_sum * num_all_remaining;

            for x in 0..=degree {
                products_sum[x] += fixed_contribution;
            }
        }

        // Contribution from remaining unfixed variables (constant term)
        if i < nv {
            let mut remaining_g_sum = F::zero();
            for j in i..nv {
                let coeffs = &prover_state.g_polys[j];
                let g_j_at_0 = coeffs[0];
                let g_j_at_1 = coeffs[0] + coeffs[1] + coeffs[2] + coeffs[3] + coeffs[4];
                remaining_g_sum += g_j_at_0 + g_j_at_1;
            }
            let num_half_remaining = F::from(1u64 << (nv - i - 1));
            let remaining_contribution = prover_state.alpha * remaining_g_sum * num_half_remaining;
            for x in 0..=degree {
                products_sum[x] += remaining_contribution;
            }
        }

        // Contribution from current variable g_{i-1}(X)
        let g_coeffs = &prover_state.g_polys[i - 1];
        let num_current_remaining = F::from(1u64 << (nv - i));

        for x in 0..=degree {
            let x_field = F::from(x as u64);
            let mut g_i_val = g_coeffs[0];
            let mut x_pow = x_field;
            for j in 1..5 {
                g_i_val += g_coeffs[j] * x_pow;
                x_pow *= x_field;
            }
            let contribution = prover_state.alpha * num_current_remaining * g_i_val;
            products_sum[x] += contribution;
        }
        #[cfg(feature = "range_proof_timing_multivariate")]
        if let Some(f) = timing {
            f(
                &format!("sumcheck round {} g_terms", round_idx),
                start_g.elapsed(),
            );
        }

        ProverMsg {
            evaluations: products_sum,
        }
    }
}
