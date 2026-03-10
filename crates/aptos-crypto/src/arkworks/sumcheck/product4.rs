// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Product-of-4-MLEs sumcheck (degree-4 round polynomials), from Jolt.

use crate::arkworks::sumcheck::{
    dense_poly::{BindingOrder, DensePolynomial},
    field::SumcheckField,
    masking::MaskingPolynomial,
    opening::{ProverOpeningAccumulator, VerifierOpeningAccumulator},
    traits::{
        OpeningAccumulator, SumcheckInstanceParams, SumcheckInstanceProver,
        SumcheckInstanceVerifier,
    },
    unipoly::UniPoly,
};
use std::marker::PhantomData;

const DEGREE: usize = 4;

/// Parameters for the product-of-4-MLE sumcheck.
pub struct Product4Params<F: SumcheckField> {
    pub num_rounds: usize,
    pub initial_claim: Option<F>,
    _marker: PhantomData<F>,
}

impl<F: SumcheckField> SumcheckInstanceParams<F> for Product4Params<F> {
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

/// Prover state for optional masking: g and challenges r_0,...,r_{j-1} for round j.
struct MaskingState<F: SumcheckField> {
    g: MaskingPolynomial<F>,
    r_prev: Vec<F::Challenge>,
}

/// Prover for sum_x M1(x)*M2(x)*M3(x)*M4(x) = claim.
/// Optionally adds a masking polynomial g (no mixed terms, degree 4) so messages are for f+g (ZK).
pub struct Product4SumcheckProver<F: SumcheckField> {
    params: Product4Params<F>,
    polys: [DensePolynomial<F>; 4],
    masking: Option<MaskingState<F>>,
}

impl<F: SumcheckField> Product4SumcheckProver<F> {
    pub fn new(num_vars: usize, evals: [Vec<F>; 4]) -> Self {
        let polys: [DensePolynomial<F>; 4] = [
            DensePolynomial::new(evals[0].clone()),
            DensePolynomial::new(evals[1].clone()),
            DensePolynomial::new(evals[2].clone()),
            DensePolynomial::new(evals[3].clone()),
        ];
        let n = polys[0].len();
        for p in &polys {
            assert_eq!(p.len(), n);
        }
        let initial_claim = (0..n).fold(F::zero(), |acc, g| {
            acc + polys[0].Z[g] * polys[1].Z[g] * polys[2].Z[g] * polys[3].Z[g]
        });
        Self {
            params: Product4Params {
                num_rounds: num_vars,
                initial_claim: Some(initial_claim),
                _marker: PhantomData,
            },
            polys,
            masking: None,
        }
    }

    /// Same as `new`, but adds a masking polynomial g (no mixed terms, degree 4).
    /// The sumcheck will prove sum of (f + g) with claim = claim_f + H_g; messages are for f+g.
    pub fn new_with_masking(num_vars: usize, evals: [Vec<F>; 4], g: MaskingPolynomial<F>) -> Self {
        assert_eq!(g.num_vars(), num_vars);
        assert!(
            g.degree() <= DEGREE,
            "masking polynomial degree must be <= {}",
            DEGREE
        );
        let mut prover = Self::new(num_vars, evals);
        let claim_f = prover.params.initial_claim.unwrap();
        let h_g = g.hypercube_sum();
        prover.params.initial_claim = Some(claim_f + h_g);
        prover.masking = Some(MaskingState {
            g,
            r_prev: Vec::with_capacity(num_vars),
        });
        prover
    }

    /// H(x) = sum_g prod_i [ (1-x)*M_i(0,g) + x*M_i(1,g) ] with (0,g) and (1,g) being the
    /// top variable (MSB), to match BindingOrder::HighToLow: pair Z[g] with Z[g+half].
    fn round_evals(polys: &[DensePolynomial<F>; 4], x_vals: &[F]) -> Vec<F> {
        let len = polys[0].len();
        assert!(len % 2 == 0);
        let half = len / 2;
        let one_minus_x: Vec<F> = x_vals.iter().map(|&x| F::one() - x).collect();
        let mut evals = vec![F::zero(); x_vals.len()];
        for g in 0..half {
            let a = [polys[0].Z[g], polys[1].Z[g], polys[2].Z[g], polys[3].Z[g]];
            let b = [
                polys[0].Z[g + half],
                polys[1].Z[g + half],
                polys[2].Z[g + half],
                polys[3].Z[g + half],
            ];
            for (k, &x) in x_vals.iter().enumerate() {
                let term = (one_minus_x[k] * a[0] + x * b[0])
                    * (one_minus_x[k] * a[1] + x * b[1])
                    * (one_minus_x[k] * a[2] + x * b[2])
                    * (one_minus_x[k] * a[3] + x * b[3]);
                evals[k] += term;
            }
        }
        evals
    }
}

impl<F: SumcheckField> SumcheckInstanceProver<F> for Product4SumcheckProver<F> {
    fn get_params(&self) -> &dyn SumcheckInstanceParams<F> {
        &self.params
    }

    fn input_claim(&self, _: &ProverOpeningAccumulator<F>) -> F {
        self.params.initial_claim.unwrap()
    }

    fn compute_message(&mut self, round: usize, _previous_claim: F) -> UniPoly<F> {
        let x_vals: Vec<F> = (0..=DEGREE).map(|i| F::from(i as u64)).collect();
        let evals = Self::round_evals(&self.polys, &x_vals);
        let mut poly_f = UniPoly::from_evals(&evals);
        if let Some(ref state) = self.masking {
            let h_j = state.g.round_message(round, &state.r_prev);
            poly_f += &h_j;
        }
        poly_f
    }

    fn ingest_challenge(&mut self, r_j: F::Challenge, _round: usize) {
        if let Some(ref mut state) = self.masking {
            state.r_prev.push(r_j);
        }
        for p in &mut self.polys {
            p.bind(&r_j, BindingOrder::HighToLow);
        }
    }
}

/// Verifier for product-of-4-MLE sumcheck.
/// When masking is used, expected output is f(r) + g(r).
pub struct Product4SumcheckVerifier<F: SumcheckField> {
    params: Product4Params<F>,
    polys_evals: Option<[Vec<F>; 4]>,
    masking: Option<MaskingPolynomial<F>>,
}

impl<F: SumcheckField> Product4SumcheckVerifier<F> {
    pub fn with_polys(num_rounds: usize, evals: [Vec<F>; 4]) -> Self {
        let n = evals[0].len();
        let initial_claim = (0..n).fold(F::zero(), |acc, g| {
            acc + evals[0][g] * evals[1][g] * evals[2][g] * evals[3][g]
        });
        Self {
            params: Product4Params {
                num_rounds,
                initial_claim: Some(initial_claim),
                _marker: PhantomData,
            },
            polys_evals: Some(evals),
            masking: None,
        }
    }

    /// Same as `with_polys`, but with masking polynomial g. Expected output is f(r) + g(r).
    /// The claim the verifier checks is claim_f + H_g (must match what the prover sent).
    pub fn with_polys_and_masking(
        num_rounds: usize,
        evals: [Vec<F>; 4],
        g: MaskingPolynomial<F>,
    ) -> Self {
        assert_eq!(g.num_vars(), num_rounds);
        let n = evals[0].len();
        let claim_f = (0..n).fold(F::zero(), |acc, idx| {
            acc + evals[0][idx] * evals[1][idx] * evals[2][idx] * evals[3][idx]
        });
        let h_g = g.hypercube_sum();
        let initial_claim = claim_f + h_g;
        Self {
            params: Product4Params {
                num_rounds,
                initial_claim: Some(initial_claim),
                _marker: PhantomData,
            },
            polys_evals: Some(evals),
            masking: Some(g),
        }
    }
}

impl<F: SumcheckField> SumcheckInstanceVerifier<F> for Product4SumcheckVerifier<F> {
    fn get_params(&self) -> &dyn SumcheckInstanceParams<F> {
        &self.params
    }

    fn expected_output_claim(
        &self,
        _accumulator: &VerifierOpeningAccumulator<F>,
        r: &[F::Challenge],
    ) -> F {
        let f_r = match &self.polys_evals {
            None => F::zero(),
            Some(evals) => {
                let mut polys: [DensePolynomial<F>; 4] = [
                    DensePolynomial::new(evals[0].clone()),
                    DensePolynomial::new(evals[1].clone()),
                    DensePolynomial::new(evals[2].clone()),
                    DensePolynomial::new(evals[3].clone()),
                ];
                for r_j in r {
                    for p in &mut polys {
                        p.bind(r_j, BindingOrder::HighToLow);
                    }
                }
                assert_eq!(polys[0].len(), 1);
                polys[0].Z[0] * polys[1].Z[0] * polys[2].Z[0] * polys[3].Z[0]
            },
        };
        let g_r = self
            .masking
            .as_ref()
            .map(|g| g.evaluate(r))
            .unwrap_or(F::zero());
        f_r + g_r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arkworks::sumcheck::{
        protocol::BatchedSumcheck, MerlinSumcheckTranscript, ProverOpeningAccumulator,
        VerifierOpeningAccumulator,
    };
    use ark_bn254::Fr;
    use ark_ff::{One, Zero};
    use merlin::Transcript;

    #[test]
    fn product4_sumcheck_prove_verify() {
        let n = 8usize; // 2^3
        let evals: [Vec<Fr>; 4] = [
            (0..n).map(|i| Fr::from(i as u64)).collect(),
            (0..n).map(|i| Fr::from((i + 1) as u64)).collect(),
            (0..n).map(|i| Fr::from((i * 2) as u64)).collect(),
            (0..n).map(|_| Fr::one()).collect(),
        ];
        let num_vars = 3;

        let mut prover = Product4SumcheckProver::<Fr>::new(num_vars, evals.clone());
        let verifier = Product4SumcheckVerifier::<Fr>::with_polys(num_vars, evals);

        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"test_sumcheck");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"test_sumcheck");
        let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);

        let (proof, _challenges, _claim) =
            BatchedSumcheck::prove(vec![&mut prover], &mut prover_acc, &mut transcript_p);

        let result = BatchedSumcheck::verify_standard::<Fr, _>(
            &proof,
            vec![&verifier],
            &mut verifier_acc,
            &mut transcript_v,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn product4_sumcheck_with_zero_masking() {
        // Zero masking: g=0 everywhere, so same as no masking. Checks that masking path doesn't break.
        let n = 8usize;
        let evals: [Vec<Fr>; 4] = [
            (0..n).map(|i| Fr::from(i as u64)).collect(),
            (0..n).map(|i| Fr::from((i + 1) as u64)).collect(),
            (0..n).map(|i| Fr::from((i * 2) as u64)).collect(),
            (0..n).map(|_| Fr::one()).collect(),
        ];
        let num_vars = 3;
        let zero = UniPoly::from_coeff(vec![Fr::zero(); DEGREE + 1]);
        let g = MaskingPolynomial::new(Fr::zero(), (0..num_vars).map(|_| zero.clone()).collect());

        let mut prover =
            Product4SumcheckProver::<Fr>::new_with_masking(num_vars, evals.clone(), g.clone());
        let verifier = Product4SumcheckVerifier::<Fr>::with_polys_and_masking(num_vars, evals, g);

        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"test_zero_masking");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"test_zero_masking");
        let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);

        let (proof, _challenges, _claim) =
            BatchedSumcheck::prove(vec![&mut prover], &mut prover_acc, &mut transcript_p);

        let result = BatchedSumcheck::verify_standard::<Fr, _>(
            &proof,
            vec![&verifier],
            &mut verifier_acc,
            &mut transcript_v,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn product4_sumcheck_with_masking() {
        let n = 8usize;
        let evals: [Vec<Fr>; 4] = [
            (0..n).map(|i| Fr::from(i as u64)).collect(),
            (0..n).map(|i| Fr::from((i + 1) as u64)).collect(),
            (0..n).map(|i| Fr::from((i * 2) as u64)).collect(),
            (0..n).map(|_| Fr::one()).collect(),
        ];
        let num_vars = 3;
        let seed = b"masking_test_seed_32_bytes!!!!!!!!";
        let g = MaskingPolynomial::<Fr>::from_seed(seed, num_vars, DEGREE);

        let mut prover =
            Product4SumcheckProver::<Fr>::new_with_masking(num_vars, evals.clone(), g.clone());
        let verifier = Product4SumcheckVerifier::<Fr>::with_polys_and_masking(num_vars, evals, g);

        let mut prover_acc = ProverOpeningAccumulator::<Fr>::new(0);
        let mut verifier_acc = VerifierOpeningAccumulator::<Fr>::new(0, false);
        let mut transcript_p_inner = Transcript::new(b"test_sumcheck_masking");
        let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
        let mut transcript_v_inner = Transcript::new(b"test_sumcheck_masking");
        let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);

        let (proof, _challenges, _claim) =
            BatchedSumcheck::prove(vec![&mut prover], &mut prover_acc, &mut transcript_p);

        let result = BatchedSumcheck::verify_standard::<Fr, _>(
            &proof,
            vec![&verifier],
            &mut verifier_acc,
            &mut transcript_v,
        );
        assert!(result.is_ok());
    }
}
