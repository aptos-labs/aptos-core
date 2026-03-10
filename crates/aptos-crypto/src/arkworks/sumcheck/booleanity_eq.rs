// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
//! Sumcheck for the polynomial
//!   F(x) = [ sum_{j=1}^m c^j * MLE[j](x) * (1 - MLE[j](x)) ] * eq_t(x) * (1 - eq_0(x))
//! where eq_t(x) = eq(x, t), eq_0(x) = eq(x, 0), and m is a fixed count (e.g. 16).
//! Here **t** is a vector of n field elements (arbitrary indices), not necessarily on the
//! boolean hypercube: eq_t(x) = ∏_i (x_i·t_i + (1−x_i)(1−t_i)) for any x.
//! The claim is sum_{x ∈ {0,1}^n} F(x); (1−eq_0(x)) excludes the origin (x=0) from the sum.
//! Adapted from the same pattern as Jolt's booleanity sumcheck (scaled MLE*(1-MLE) with eq factors).

use crate::arkworks::sumcheck::{
    dense_poly::{BindingOrder, DensePolynomial},
    field::SumcheckField,
    opening::{ProverOpeningAccumulator, VerifierOpeningAccumulator},
    traits::{
        OpeningAccumulator, SumcheckInstanceParams, SumcheckInstanceProver,
        SumcheckInstanceVerifier,
    },
    unipoly::UniPoly,
};
use std::marker::PhantomData;

// This is the degree of F(x)
const DEGREE: usize = 4;

/// Parameters for the booleanity-eq sumcheck.
pub struct BooleanityEqParams<F: SumcheckField> {
    pub num_rounds: usize,
    pub initial_claim: Option<F>,
    _marker: PhantomData<F>,
}

impl<F: SumcheckField> SumcheckInstanceParams<F> for BooleanityEqParams<F> {
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

/// Prover for sum_x F(x) where
/// F(x) = [ sum_j c^j * MLE[j](x)*(1-MLE[j](x)) ] * eq_t(x) * (1 - eq_0(x)).
/// **t** is a vector of n field elements (arbitrary indices).
pub struct BooleanityEqSumcheckProver<F: SumcheckField> {
    params: BooleanityEqParams<F>,
    polys: Vec<DensePolynomial<F>>,
    c_powers: Vec<F>,
    #[allow(dead_code)]
    t: Vec<F>,
    eq_t_evals: Vec<F>,
    /// (1 - eq_0(x)) evals. Initially 0 at origin, 1 elsewhere; we store and bind each round
    /// because after folding the values are no longer that simple pattern.
    one_minus_eq_0_evals: Vec<F>,
}

impl<F: SumcheckField> BooleanityEqSumcheckProver<F> {
    /// Build prover from m MLEs (each 2^n evals), scalar c, and **t** as a vector of n field
    /// elements (arbitrary indices). eq_t(x) = ∏_i (x_i·t_i + (1−x_i)(1−t_i)) for x on the
    /// hypercube (variable order: first round = MSB).
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

        // eq_t(g) = ∏_i (g_i ? t[i] : (1−t[i])) with g_i = MSB-first bit of g
        let eq_t_evals: Vec<F> = (0..n)
            .map(|g| {
                (0..num_vars)
                    .map(|i| {
                        let bit = (g >> (num_vars - 1 - i)) & 1;
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

        // Claim: sum over the boolean hypercube {0,1}^n (g indexes the 2^n points, MSB-first).
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
            params: BooleanityEqParams {
                num_rounds: num_vars,
                initial_claim: Some(initial_claim),
                _marker: PhantomData,
            },
            polys,
            c_powers,
            t,
            eq_t_evals,
            one_minus_eq_0_evals,
        }
    }

    /// Convenience: build prover when t is a hypercube point given by index in [0, 2^n), t ≠ 0.
    /// TODO: remove?
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
                if (t_index >> (num_vars - 1 - i)) & 1 == 1 {
                    F::one()
                } else {
                    F::zero()
                }
            })
            .collect();
        Self::new(num_vars, mle_evals, c, t)
    }

    /// Round polynomial evaluations: for x_cur in {0,1,2,3,4}, sum over the rest of the hypercube.
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

        for g in 0..half {
            let eq_t_0 = eq_t_evals[g];
            let eq_t_1 = eq_t_evals[g + half];
            let om0_0 = one_minus_eq_0_evals[g];
            let om0_1 = one_minus_eq_0_evals[g + half];

            for (k, &x) in x_vals.iter().enumerate() {
                let one_minus_x = F::one() - x;

                let mut inner = F::zero();
                for (j, p) in polys.iter().enumerate() {
                    let a = p.Z[g];
                    let b = p.Z[g + half];
                    let mle_x = one_minus_x * a + x * b;
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

    fn bind_evals(evals: &mut Vec<F>, r: &F::Challenge, _order: BindingOrder) {
        let r_f = F::challenge_to_field(r);
        let n = evals.len() / 2;
        let (left, right) = evals.split_at_mut(n);
        for (a, b) in left.iter_mut().zip(right.iter()) {
            *a += r_f * (*b - *a);
        }
        evals.truncate(n);
    }
}

impl<F: SumcheckField> SumcheckInstanceProver<F> for BooleanityEqSumcheckProver<F> {
    fn get_params(&self) -> &dyn SumcheckInstanceParams<F> {
        &self.params
    }

    fn input_claim(&self, _: &ProverOpeningAccumulator<F>) -> F {
        self.params.initial_claim.unwrap()
    }

    fn compute_message(&mut self, _round: usize, _previous_claim: F) -> UniPoly<F> {
        let x_vals: Vec<F> = (0..=DEGREE).map(|i| F::from(i as u64)).collect();
        let evals = Self::round_evals(
            &self.polys,
            &self.c_powers,
            &self.eq_t_evals,
            &self.one_minus_eq_0_evals,
            &x_vals,
        );
        UniPoly::from_evals(&evals)
    }

    fn ingest_challenge(&mut self, r_j: F::Challenge, _round: usize) {
        for p in &mut self.polys {
            p.bind(&r_j, BindingOrder::HighToLow);
        }
        Self::bind_evals(&mut self.eq_t_evals, &r_j, BindingOrder::HighToLow);
        Self::bind_evals(
            &mut self.one_minus_eq_0_evals,
            &r_j,
            BindingOrder::HighToLow,
        );
    }
}

/// Verifier for the booleanity-eq sumcheck.
pub struct BooleanityEqSumcheckVerifier<F: SumcheckField> {
    params: BooleanityEqParams<F>,
    polys_evals: Option<Vec<Vec<F>>>,
    #[allow(dead_code)]
    c: F,
    c_powers: Option<Vec<F>>,
    t: Vec<F>,
}

impl<F: SumcheckField> BooleanityEqSumcheckVerifier<F> {
    /// **t** is a vector of n field elements (arbitrary indices), same as the prover.
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
                        let bit = (g >> (num_rounds - 1 - i)) & 1;
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
            params: BooleanityEqParams {
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

    /// Convenience: verifier when t is a hypercube point given by index in [0, 2^n), t ≠ 0.
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
                if (t_index >> (num_rounds - 1 - i)) & 1 == 1 {
                    F::one()
                } else {
                    F::zero()
                }
            })
            .collect();
        Self::new(num_rounds, mle_evals, c, t)
    }

    /// eq(r, t) = ∏_i (r_i·t_i + (1−r_i)(1−t_i)) for arbitrary t.
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

    /// eq(r, 0) = prod_i (1 - r_i).
    fn eq_zero(r: &[F::Challenge]) -> F {
        r.iter()
            .map(|ri| F::one() - F::challenge_to_field(ri))
            .fold(F::one(), |a, b| a * b)
    }
}

impl<F: SumcheckField> SumcheckInstanceVerifier<F> for BooleanityEqSumcheckVerifier<F> {
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
                p.bind(r_j, BindingOrder::HighToLow);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arkworks::sumcheck::{
        opening::{ProverOpeningAccumulator, VerifierOpeningAccumulator},
        protocol::BatchedSumcheck,
        MerlinSumcheckTranscript,
    };
    use ark_bn254::Fr;
    use ark_ff::{One, UniformRand, Zero};
    use merlin::Transcript;

    #[test]
    fn booleanity_eq_sumcheck_prove_verify_arbitrary_t() {
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
            BooleanityEqSumcheckProver::<Fr>::new(num_vars, mle_evals.clone(), c, t.clone());
        let verifier = BooleanityEqSumcheckVerifier::<Fr>::new(num_vars, mle_evals.clone(), c, t);

        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"booleanity_eq_test");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"booleanity_eq_test");
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
    fn booleanity_eq_sumcheck_one_var() {
        // Minimal: n=2, one variable, so one round.
        let mut rng = ark_std::test_rng();
        let num_vars = 1;
        let n = 2;
        let m = 2usize;
        let c = Fr::rand(&mut rng);
        let t = vec![Fr::rand(&mut rng)];

        let mle_evals: Vec<Vec<Fr>> = (0..m)
            .map(|_| (0..n).map(|_| Fr::rand(&mut rng)).collect())
            .collect();

        let prover_claim = {
            let eq_t_0 = Fr::one() - t[0];
            let eq_t_1 = t[0];
            let om0_0 = Fr::zero();
            let om0_1 = Fr::one();
            (0..n)
                .map(|g| {
                    let mut inner = Fr::zero();
                    let mut c_j = c;
                    for j in 0..m {
                        let v = mle_evals[j][g];
                        inner += c_j * v * (Fr::one() - v);
                        c_j *= c;
                    }
                    let eq_t_g = if g == 0 { eq_t_0 } else { eq_t_1 };
                    let om0_g = if g == 0 { om0_0 } else { om0_1 };
                    inner * eq_t_g * om0_g
                })
                .fold(Fr::zero(), |a, b| a + b)
        };

        let prover =
            BooleanityEqSumcheckProver::<Fr>::new(num_vars, mle_evals.clone(), c, t.clone());
        assert_eq!(prover.params.initial_claim.unwrap(), prover_claim);

        let verifier =
            BooleanityEqSumcheckVerifier::<Fr>::new(num_vars, mle_evals.clone(), c, t.clone());
        assert_eq!(verifier.params.initial_claim.unwrap(), prover_claim);

        // Full prove/verify with one variable
        let mut prover =
            BooleanityEqSumcheckProver::<Fr>::new(num_vars, mle_evals.clone(), c, t.clone());
        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"one_var");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"one_var");
        let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);
        let (proof, _challenges, _) =
            BatchedSumcheck::prove(vec![&mut prover], &mut prover_acc, &mut transcript_p);
        let verifier = BooleanityEqSumcheckVerifier::<Fr>::new(num_vars, mle_evals, c, t);
        let result = BatchedSumcheck::verify_standard::<Fr, _>(
            &proof,
            vec![&verifier],
            &mut verifier_acc,
            &mut transcript_v,
        );
        assert!(result.is_ok(), "one-round prove/verify failed");
    }

    #[test]
    fn booleanity_eq_sumcheck_two_vars() {
        let mut rng = ark_std::test_rng();
        let num_vars = 2;
        let n = 4;
        let m = 2usize;
        let c = Fr::rand(&mut rng);
        let t = vec![Fr::rand(&mut rng), Fr::rand(&mut rng)];

        let mle_evals: Vec<Vec<Fr>> = (0..m)
            .map(|_| (0..n).map(|_| Fr::rand(&mut rng)).collect())
            .collect();

        let mut prover =
            BooleanityEqSumcheckProver::<Fr>::new(num_vars, mle_evals.clone(), c, t.clone());
        let verifier = BooleanityEqSumcheckVerifier::<Fr>::new(num_vars, mle_evals.clone(), c, t);

        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"two_var");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"two_var");
        let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);

        let (proof, _challenges, _) =
            BatchedSumcheck::prove(vec![&mut prover], &mut prover_acc, &mut transcript_p);

        let result = BatchedSumcheck::verify_standard::<Fr, _>(
            &proof,
            vec![&verifier],
            &mut verifier_acc,
            &mut transcript_v,
        );
        assert!(result.is_ok(), "two-round prove/verify failed");
    }

    #[test]
    fn booleanity_eq_sumcheck_prove_verify_hypercube_t() {
        let mut rng = ark_std::test_rng();
        let num_vars = 4;
        let n = 1 << num_vars;
        let m = 8usize;
        let c = Fr::rand(&mut rng);
        let t_index = 7usize; // != 0, on hypercube

        let mle_evals: Vec<Vec<Fr>> = (0..m)
            .map(|_| (0..n).map(|_| Fr::rand(&mut rng)).collect())
            .collect();

        let mut prover = BooleanityEqSumcheckProver::<Fr>::new_with_hypercube_point(
            num_vars,
            mle_evals.clone(),
            c,
            t_index,
        );
        let verifier = BooleanityEqSumcheckVerifier::<Fr>::new_with_hypercube_point(
            num_vars,
            mle_evals.clone(),
            c,
            t_index,
        );

        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"booleanity_eq_hypercube");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"booleanity_eq_hypercube");
        let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);

        let (proof, challenges, _claim) =
            BatchedSumcheck::prove(vec![&mut prover], &mut prover_acc, &mut transcript_p);

        let result = BatchedSumcheck::verify_standard::<Fr, _>(
            &proof,
            vec![&verifier],
            &mut verifier_acc,
            &mut transcript_v,
        );
        assert!(result.is_ok());
        assert_eq!(challenges.len(), num_vars);

        // When t is on hypercube, sum_x F(x) = F(t) = sum_{j=1}^m c^j * MLE[j](t)*(1-MLE[j](t))
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
