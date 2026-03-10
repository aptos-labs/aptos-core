// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//
//! Sumcheck for the same polynomial as [super::booleanity_eq], but with **LSB-first** variable order.
//!
//! F(x) = [ sum_j c^j * MLE[j](x)*(1-MLE[j](x)) ] * eq_t(x) * (1 - eq_0(x)).
//! Here the **first round** binds the **least significant bit** (variable 0 = LSB), matching
//! aptos-dkg and ark_poly (DenseMultilinearExtension) convention. Same proof size and round count
//! as the MSB version; only the binding order and eq_t indexing change.

use crate::arkworks::sumcheck::{
    dense_poly::{BindingOrder, DensePolynomial},
    field::SumcheckField,
    masking::MaskingPolynomial,
    opening::VerifierOpeningAccumulator,
    traits::{
        OpeningAccumulator, SumcheckInstanceParams, SumcheckInstanceProver,
        SumcheckInstanceVerifier,
    },
    unipoly::UniPoly,
    ProverOpeningAccumulator,
};
use std::marker::PhantomData;

const DEGREE: usize = 4;

/// Prover state for optional masking: alpha * g and challenges r_0,...,r_{j-1} for round j (LSB order).
struct MaskingState<F: SumcheckField> {
    alpha: F,
    g: MaskingPolynomial<F>,
    r_prev: Vec<F::Challenge>,
}

/// Parameters for the booleanity-eq sumcheck (LSB-first).
pub struct BooleanityEqLsbParams<F: SumcheckField> {
    pub num_rounds: usize,
    pub initial_claim: Option<F>,
    _marker: PhantomData<F>,
}

impl<F: SumcheckField> SumcheckInstanceParams<F> for BooleanityEqLsbParams<F> {
    fn degree(&self) -> usize {
        DEGREE
    }

    fn num_rounds(&self) -> usize {
        self.num_rounds
    }

    fn input_claim(&self, _: &dyn OpeningAccumulator<F>) -> F {
        self.initial_claim.unwrap_or(F::zero())
    }
}

/// Prover for sum_x F(x) with **LSB-first** variable order (variable 0 = LSB).
/// Same polynomial as [super::BooleanityEqSumcheckProver]; evals are in natural index order
/// (index g = bits in natural order); we bind LSB first via (2b, 2b+1) folding.
pub struct BooleanityEqSumcheckProverLSB<F: SumcheckField> {
    params: BooleanityEqLsbParams<F>,
    polys: Vec<DensePolynomial<F>>,
    #[allow(dead_code)]
    c: F,
    c_powers: Vec<F>,
    #[allow(dead_code)]
    t: Vec<F>,
    eq_t_evals: Vec<F>,
    one_minus_eq_0_evals: Vec<F>,
    /// Optional masking: alpha * g(X) added to the polynomial (claim += alpha * H_g).
    masking: Option<MaskingState<F>>,
}

impl<F: SumcheckField> BooleanityEqSumcheckProverLSB<F> {
    /// Build prover from m MLEs (each 2^n evals), scalar c, and **t** (n field elements).
    /// Variable order: **first round = LSB** (eq_t(g) uses g_i = (g >> i) & 1).
    pub fn new(num_vars: usize, mle_evals: Vec<Vec<F>>, c: F, t: Vec<F>) -> Self {
        let n = 1 << num_vars;
        assert_eq!(t.len(), num_vars, "t must have length num_vars");
        assert!(!mle_evals.is_empty());
        for evals in &mle_evals {
            assert_eq!(evals.len(), n);
        }

        let polys: Vec<DensePolynomial<F>> = mle_evals
            .iter()
            .map(|evals| DensePolynomial::new(evals.clone()))
            .collect();
        let m = polys.len();

        let mut c_powers = Vec::with_capacity(m);
        let mut c_j = c;
        for _ in 0..m {
            c_powers.push(c_j);
            c_j *= c;
        }

        // eq_t(g) = ∏_i (g_i ? t[i] : (1−t[i])) with g_i = LSB-first bit of g: bit i = (g >> i) & 1
        let eq_t_evals: Vec<F> = (0..n)
            .map(|g| {
                (0..num_vars)
                    .map(|i| {
                        let bit = (g >> i) & 1;
                        if bit == 1 {
                            t[i]
                        } else {
                            F::one() - t[i]
                        }
                    })
                    .fold(F::one(), |a, b| a * b)
            })
            .collect();

        let mut one_minus_eq_0_evals = vec![F::one(); n];
        one_minus_eq_0_evals[0] = F::zero();

        let initial_claim = (0..n)
            .map(|g| {
                let eq_t_g = eq_t_evals[g];
                let om0_g = one_minus_eq_0_evals[g];
                let inner: F = (0..m)
                    .map(|j| {
                        let v = polys[j].Z[g];
                        c_powers[j] * v * (F::one() - v)
                    })
                    .fold(F::zero(), |a, b| a + b);
                inner * eq_t_g * om0_g
            })
            .fold(F::zero(), |a, b| a + b);

        Self {
            params: BooleanityEqLsbParams {
                num_rounds: num_vars,
                initial_claim: Some(initial_claim),
                _marker: PhantomData,
            },
            polys,
            c,
            c_powers,
            t,
            eq_t_evals,
            one_minus_eq_0_evals,
            masking: None,
        }
    }

    /// Same as `new`, but adds a masking polynomial g (no mixed terms, degree ≤ 4) scaled by alpha.
    /// The sumcheck proves sum of (BooleanityEq + alpha*g) with claim = claim_bool + alpha * H_g.
    pub fn new_with_masking(
        num_vars: usize,
        mle_evals: Vec<Vec<F>>,
        c: F,
        t: Vec<F>,
        alpha: F,
        g: MaskingPolynomial<F>,
    ) -> Self {
        assert_eq!(g.num_vars(), num_vars);
        assert!(
            g.degree() <= DEGREE,
            "masking polynomial degree must be <= {}",
            DEGREE
        );
        let mut prover = Self::new(num_vars, mle_evals, c, t);
        let claim_bool = prover.params.initial_claim.unwrap();
        let h_g = g.hypercube_sum();
        prover.params.initial_claim = Some(claim_bool + alpha * h_g);
        prover.masking = Some(MaskingState {
            alpha,
            g,
            r_prev: Vec::with_capacity(num_vars),
        });
        prover
    }

    /// Build prover when t is a hypercube point by index in [0, 2^n), t ≠ 0 (LSB-first bits).
    pub fn new_with_hypercube_point(
        num_vars: usize,
        mle_evals: Vec<Vec<F>>,
        c: F,
        t_index: usize,
    ) -> Self {
        let n = 1 << num_vars;
        assert!(t_index < n && t_index != 0);
        let t: Vec<F> = (0..num_vars)
            .map(|i| {
                if (t_index >> i) & 1 == 1 {
                    F::one()
                } else {
                    F::zero()
                }
            })
            .collect();
        Self::new(num_vars, mle_evals, c, t)
    }

    /// Round polynomial: pair (2b, 2b+1) for LSB binding (same as dkg (b<<1, (b<<1)+1)).
    fn round_evals(
        polys: &[DensePolynomial<F>],
        c_powers: &[F],
        eq_t_evals: &[F],
        one_minus_eq_0_evals: &[F],
        x_vals: &[F],
    ) -> Vec<F> {
        let len = polys[0].len;
        assert!(len >= 2);
        let half = len / 2;
        let mut evals = vec![F::zero(); x_vals.len()];

        for b in 0..half {
            let eq_t_0 = eq_t_evals[2 * b];
            let eq_t_1 = eq_t_evals[2 * b + 1];
            let om0_0 = one_minus_eq_0_evals[2 * b];
            let om0_1 = one_minus_eq_0_evals[2 * b + 1];

            for (k, &x) in x_vals.iter().enumerate() {
                let one_minus_x = F::one() - x;

                let mut inner = F::zero();
                for (j, p) in polys.iter().enumerate() {
                    let a = p.Z[2 * b];
                    let b_val = p.Z[2 * b + 1];
                    let mle_x = one_minus_x * a + x * b_val;
                    let bool_j = mle_x * (F::one() - mle_x);
                    inner += c_powers[j] * bool_j;
                }

                let eq_t_x = one_minus_x * eq_t_0 + x * eq_t_1;
                let om0_x = one_minus_x * om0_0 + x * om0_1;
                evals[k] += inner * eq_t_x * om0_x;
            }
        }
        evals
    }

    /// Bind evals for LSB: fold even/odd (evals[2i], evals[2i+1]).
    fn bind_evals_lsb(evals: &mut Vec<F>, r: &F::Challenge) {
        let r_f = F::challenge_to_field(r);
        let n = evals.len() / 2;
        let old: Vec<F> = (0..n)
            .map(|i| {
                let a = evals[2 * i];
                let b = evals[2 * i + 1];
                a + r_f * (b - a)
            })
            .collect();
        evals.clear();
        evals.extend(old);
    }
}

impl<F: SumcheckField> SumcheckInstanceProver<F> for BooleanityEqSumcheckProverLSB<F> {
    fn get_params(&self) -> &dyn SumcheckInstanceParams<F> {
        &self.params
    }

    fn input_claim(&self, _: &ProverOpeningAccumulator<F>) -> F {
        self.params.initial_claim.unwrap()
    }

    fn compute_message(&mut self, round: usize, _previous_claim: F) -> UniPoly<F> {
        let x_vals: Vec<F> = (0..=DEGREE).map(|i| F::from(i as u64)).collect();
        let evals = Self::round_evals(
            &self.polys,
            &self.c_powers,
            &self.eq_t_evals,
            &self.one_minus_eq_0_evals,
            &x_vals,
        );
        let mut poly = UniPoly::from_evals(&evals);
        if let Some(ref state) = self.masking {
            let h_j = state.g.round_message(round, &state.r_prev);
            poly += &(&h_j * state.alpha);
        }
        poly
    }

    fn ingest_challenge(&mut self, r_j: F::Challenge, _round: usize) {
        if let Some(ref mut state) = self.masking {
            state.r_prev.push(r_j);
        }
        for p in &mut self.polys {
            p.bind(&r_j, BindingOrder::LowToHigh);
        }
        Self::bind_evals_lsb(&mut self.eq_t_evals, &r_j);
        Self::bind_evals_lsb(&mut self.one_minus_eq_0_evals, &r_j);
    }
}

/// Verifier for booleanity-eq sumcheck with **LSB-first** variable order.
pub struct BooleanityEqSumcheckVerifierLSB<F: SumcheckField> {
    params: BooleanityEqLsbParams<F>,
    polys_evals: Option<Vec<Vec<F>>>,
    #[allow(dead_code)]
    c: F,
    c_powers: Option<Vec<F>>,
    t: Vec<F>,
}

impl<F: SumcheckField> BooleanityEqSumcheckVerifierLSB<F> {
    pub fn new(num_rounds: usize, mle_evals: Vec<Vec<F>>, c: F, t: Vec<F>) -> Self {
        let n = 1 << num_rounds;
        assert_eq!(t.len(), num_rounds, "t must have length num_rounds");
        for evals in &mle_evals {
            assert_eq!(evals.len(), n);
        }
        let m = mle_evals.len();
        let mut c_powers = Vec::with_capacity(m);
        let mut c_j = c;
        for _ in 0..m {
            c_powers.push(c_j);
            c_j *= c;
        }
        let polys: Vec<DensePolynomial<F>> = mle_evals
            .iter()
            .map(|evals| DensePolynomial::new(evals.clone()))
            .collect();
        let eq_t_evals: Vec<F> = (0..n)
            .map(|g| {
                (0..num_rounds)
                    .map(|i| {
                        let bit = (g >> i) & 1;
                        if bit == 1 {
                            t[i]
                        } else {
                            F::one() - t[i]
                        }
                    })
                    .fold(F::one(), |a, b| a * b)
            })
            .collect();
        let mut one_minus_eq_0 = vec![F::one(); n];
        one_minus_eq_0[0] = F::zero();
        let initial_claim = (0..n)
            .map(|g| {
                let inner: F = (0..m)
                    .map(|j| {
                        let v = polys[j].Z[g];
                        c_powers[j] * v * (F::one() - v)
                    })
                    .fold(F::zero(), |a, b| a + b);
                inner * eq_t_evals[g] * one_minus_eq_0[g]
            })
            .fold(F::zero(), |a, b| a + b);

        Self {
            params: BooleanityEqLsbParams {
                num_rounds,
                initial_claim: Some(initial_claim),
                _marker: PhantomData,
            },
            polys_evals: Some(mle_evals),
            c,
            c_powers: Some(c_powers),
            t,
        }
    }

    pub fn new_with_hypercube_point(
        num_rounds: usize,
        mle_evals: Vec<Vec<F>>,
        c: F,
        t_index: usize,
    ) -> Self {
        let n = 1 << num_rounds;
        assert!(t_index < n && t_index != 0);
        let t: Vec<F> = (0..num_rounds)
            .map(|i| {
                if (t_index >> i) & 1 == 1 {
                    F::one()
                } else {
                    F::zero()
                }
            })
            .collect();
        Self::new(num_rounds, mle_evals, c, t)
    }

    fn eq_at_point(r: &[F::Challenge], t: &[F]) -> F {
        assert_eq!(r.len(), t.len());
        r.iter()
            .zip(t.iter())
            .map(|(ri, ti)| {
                let r_i = F::challenge_to_field(ri);
                r_i * *ti + (F::one() - r_i) * (F::one() - *ti)
            })
            .fold(F::one(), |a, b| a * b)
    }

    fn eq_zero(r: &[F::Challenge]) -> F {
        r.iter()
            .map(|ri| F::one() - F::challenge_to_field(ri))
            .fold(F::one(), |a, b| a * b)
    }
}

impl<F: SumcheckField> SumcheckInstanceVerifier<F> for BooleanityEqSumcheckVerifierLSB<F> {
    fn get_params(&self) -> &dyn SumcheckInstanceParams<F> {
        &self.params
    }

    fn expected_output_claim(
        &self,
        _accumulator: &VerifierOpeningAccumulator<F>,
        r: &[F::Challenge],
    ) -> F {
        let evals = match &self.polys_evals {
            None => return F::zero(),
            Some(e) => e,
        };
        let c_powers = match &self.c_powers {
            None => return F::zero(),
            Some(p) => p,
        };

        let mut polys: Vec<DensePolynomial<F>> = evals
            .iter()
            .map(|evals| DensePolynomial::new(evals.clone()))
            .collect();
        for r_j in r {
            for p in &mut polys {
                p.bind(r_j, BindingOrder::LowToHigh);
            }
        }
        assert_eq!(polys[0].len, 1);

        let eq_t_r = Self::eq_at_point(r, &self.t);
        let eq_0_r = Self::eq_zero(r);
        let one_minus_eq_0_r = F::one() - eq_0_r;

        let inner: F = polys
            .iter()
            .zip(c_powers.iter())
            .map(|(p, c_j)| {
                let v = p.Z[0];
                *c_j * v * (F::one() - v)
            })
            .fold(F::zero(), |a, b| a + b);

        inner * eq_t_r * one_minus_eq_0_r
    }
}

/// Verifier for booleanity-eq sumcheck (LSB) when the verifier only has the MLE
/// evaluations at the sumcheck point (e.g. y_js from the range proof), not the full MLE evals.
/// Use this when the full mle_evals are not available to the verifier.
/// Optionally includes alpha * y_g in the expected output (for the mask term in Dekart).
pub struct BooleanityEqSumcheckVerifierLSBWithOpenings<F: SumcheckField> {
    params: BooleanityEqLsbParams<F>,
    c_powers: Vec<F>,
    t: Vec<F>,
    /// MLE evaluations at the sumcheck point r (e.g. y_1,...,y_ell from the proof).
    openings: Vec<F>,
    /// If present, expected_output_claim adds alpha * y_g (for Dekart's alpha*g mask term).
    alpha_y_g: Option<F>,
}

impl<F: SumcheckField> BooleanityEqSumcheckVerifierLSBWithOpenings<F> {
    /// Create verifier with (num_rounds, c, t, initial_claim, openings).
    /// `openings` are the MLE evaluations at the sumcheck point (e.g. y_js from the proof).
    pub fn new(num_rounds: usize, c: F, t: Vec<F>, initial_claim: F, openings: Vec<F>) -> Self {
        Self::new_with_alpha_y_g(num_rounds, c, t, initial_claim, openings, None)
    }

    /// Same as `new` but with optional alpha * y_g added to expected output (for Dekart).
    pub fn new_with_alpha_y_g(
        num_rounds: usize,
        c: F,
        t: Vec<F>,
        initial_claim: F,
        openings: Vec<F>,
        alpha_y_g: Option<F>,
    ) -> Self {
        assert_eq!(t.len(), num_rounds, "t must have length num_rounds");
        let m = openings.len();
        let mut c_powers = Vec::with_capacity(m);
        let mut c_j = c;
        for _ in 0..m {
            c_powers.push(c_j);
            c_j *= c;
        }
        Self {
            params: BooleanityEqLsbParams {
                num_rounds,
                initial_claim: Some(initial_claim),
                _marker: PhantomData,
            },
            c_powers,
            t,
            openings,
            alpha_y_g,
        }
    }

    fn eq_at_point(r: &[F::Challenge], t: &[F]) -> F {
        assert_eq!(r.len(), t.len());
        r.iter()
            .zip(t.iter())
            .map(|(ri, ti)| {
                let r_i = F::challenge_to_field(ri);
                r_i * *ti + (F::one() - r_i) * (F::one() - *ti)
            })
            .fold(F::one(), |a, b| a * b)
    }

    fn eq_zero(r: &[F::Challenge]) -> F {
        r.iter()
            .map(|ri| F::one() - F::challenge_to_field(ri))
            .fold(F::one(), |a, b| a * b)
    }
}

impl<F: SumcheckField> SumcheckInstanceVerifier<F>
    for BooleanityEqSumcheckVerifierLSBWithOpenings<F>
{
    fn get_params(&self) -> &dyn SumcheckInstanceParams<F> {
        &self.params
    }

    fn expected_output_claim(
        &self,
        _accumulator: &VerifierOpeningAccumulator<F>,
        r: &[F::Challenge],
    ) -> F {
        let eq_t_r = Self::eq_at_point(r, &self.t);
        let eq_0_r = Self::eq_zero(r);
        let one_minus_eq_0_r = F::one() - eq_0_r;
        let inner: F = self
            .openings
            .iter()
            .zip(self.c_powers.iter())
            .map(|(v, c_j)| *c_j * *v * (F::one() - *v))
            .fold(F::zero(), |a, b| a + b);
        let base = inner * eq_t_r * one_minus_eq_0_r;
        self.alpha_y_g.map(|a| base + a).unwrap_or(base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arkworks::sumcheck::{
        opening::{ProverOpeningAccumulator, VerifierOpeningAccumulator},
        protocol::BatchedSumcheck,
        traits::SumcheckInstanceProver,
        MerlinSumcheckTranscript,
    };
    use ark_bn254::Fr;
    use ark_ff::{One, UniformRand, Zero};
    use merlin::Transcript;

    #[test]
    fn booleanity_eq_lsb_prove_verify_arbitrary_t() {
        let mut rng = ark_std::test_rng();
        let num_vars = 4;
        let n = 1 << num_vars;
        let m = 8usize;
        let c = Fr::rand(&mut rng);
        let t: Vec<Fr> = (0..num_vars).map(|_| Fr::rand(&mut rng)).collect();

        let mle_evals: Vec<Vec<Fr>> = (0..m)
            .map(|_| (0..n).map(|_| Fr::rand(&mut rng)).collect())
            .collect();

        let mut prover =
            BooleanityEqSumcheckProverLSB::<Fr>::new(num_vars, mle_evals.clone(), c, t.clone());
        let verifier =
            BooleanityEqSumcheckVerifierLSB::<Fr>::new(num_vars, mle_evals.clone(), c, t);

        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"booleanity_eq_lsb_test");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"booleanity_eq_lsb_test");
        let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);

        let (proof, challenges, _claim) =
            BatchedSumcheck::prove(vec![&mut prover], &mut prover_acc, &mut transcript_p);

        let result = BatchedSumcheck::verify_standard::<Fr, _>(
            &proof,
            vec![&verifier],
            &mut verifier_acc,
            &mut transcript_v,
        );
        assert!(result.is_ok(), "verify failed (arbitrary t)");
        assert_eq!(challenges.len(), num_vars);
    }

    #[test]
    fn booleanity_eq_lsb_same_claim_as_msb_for_same_evals() {
        // For the same mle_evals and the *same hypercube point* t (expressed in each convention),
        // the polynomial F is the same, so the hypercube sum agrees; only the binding order differs.
        // MSB: t[i] = bit (num_vars-1-i) of index; LSB: t[i] = bit i of index.
        let mut rng = ark_std::test_rng();
        let num_vars = 4;
        let n = 1 << num_vars;
        let m = 4usize;
        let c = Fr::rand(&mut rng);
        let t_index = 7usize; // same point on the hypercube
        let t_msb: Vec<Fr> = (0..num_vars)
            .map(|i| {
                if (t_index >> (num_vars - 1 - i)) & 1 == 1 {
                    Fr::one()
                } else {
                    Fr::zero()
                }
            })
            .collect();
        let t_lsb: Vec<Fr> = (0..num_vars)
            .map(|i| {
                if (t_index >> i) & 1 == 1 {
                    Fr::one()
                } else {
                    Fr::zero()
                }
            })
            .collect();
        let mle_evals: Vec<Vec<Fr>> = (0..m)
            .map(|_| (0..n).map(|_| Fr::rand(&mut rng)).collect())
            .collect();

        let prover_msb = crate::arkworks::sumcheck::BooleanityEqSumcheckProver::new(
            num_vars,
            mle_evals.clone(),
            c,
            t_msb,
        );
        let prover_lsb =
            BooleanityEqSumcheckProverLSB::<Fr>::new(num_vars, mle_evals.clone(), c, t_lsb);

        let acc = ProverOpeningAccumulator::<Fr>::new(0);
        let claim_msb = prover_msb.get_params().input_claim(&acc);
        let claim_lsb = prover_lsb.get_params().input_claim(&acc);
        assert_eq!(
            claim_msb,
            claim_lsb,
            "LSB and MSB must agree on the same hypercube sum when t is the same point (in each convention)"
        );
    }

    #[test]
    fn booleanity_eq_lsb_hypercube_t() {
        let mut rng = ark_std::test_rng();
        let num_vars = 4;
        let n = 1 << num_vars;
        let m = 8usize;
        let c = Fr::rand(&mut rng);
        let t_index = 7usize;

        let mle_evals: Vec<Vec<Fr>> = (0..m)
            .map(|_| (0..n).map(|_| Fr::rand(&mut rng)).collect())
            .collect();

        let mut prover = BooleanityEqSumcheckProverLSB::new_with_hypercube_point(
            num_vars,
            mle_evals.clone(),
            c,
            t_index,
        );
        let verifier = BooleanityEqSumcheckVerifierLSB::new_with_hypercube_point(
            num_vars,
            mle_evals.clone(),
            c,
            t_index,
        );

        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"booleanity_eq_lsb_hypercube");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"booleanity_eq_lsb_hypercube");
        let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);

        let (proof, _challenges, _) =
            BatchedSumcheck::prove(vec![&mut prover], &mut prover_acc, &mut transcript_p);

        let result = BatchedSumcheck::verify_standard::<Fr, _>(
            &proof,
            vec![&verifier],
            &mut verifier_acc,
            &mut transcript_v,
        );
        assert!(result.is_ok());

        let mut expected_claim = Fr::zero();
        let mut c_j = c;
        for j in 0..m {
            let v = mle_evals[j][t_index];
            expected_claim += c_j * v * (Fr::one() - v);
            c_j *= c;
        }
        assert_eq!(
            prover.params.initial_claim.unwrap(),
            expected_claim,
            "hypercube claim should equal F(t)"
        );
    }
}
