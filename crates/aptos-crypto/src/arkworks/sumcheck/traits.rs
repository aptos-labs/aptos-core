// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Sumcheck instance traits (from Jolt, no jolt dep).

use crate::arkworks::sumcheck::{
    field::SumcheckField,
    opening::{ProverOpeningAccumulator, VerifierOpeningAccumulator},
    unipoly::UniPoly,
};

/// Minimal accumulator marker (both prover and verifier accumulators implement this).
pub trait OpeningAccumulator<F: SumcheckField> {}

impl<F: SumcheckField> OpeningAccumulator<F> for ProverOpeningAccumulator<F> {}
impl<F: SumcheckField> OpeningAccumulator<F> for VerifierOpeningAccumulator<F> {}

/// Parameters shared by prover and verifier for one sumcheck instance.
pub trait SumcheckInstanceParams<F: SumcheckField> {
    fn degree(&self) -> usize;
    fn num_rounds(&self) -> usize;
    fn input_claim(&self, accumulator: &dyn OpeningAccumulator<F>) -> F;
}

/// Prover for one sumcheck instance.
pub trait SumcheckInstanceProver<F: SumcheckField> {
    fn get_params(&self) -> &dyn SumcheckInstanceParams<F>;
    fn degree(&self) -> usize {
        self.get_params().degree()
    }
    fn num_rounds(&self) -> usize {
        self.get_params().num_rounds()
    }
    fn round_offset(&self, max_num_rounds: usize) -> usize {
        max_num_rounds - self.num_rounds()
    }
    fn input_claim(&self, accumulator: &ProverOpeningAccumulator<F>) -> F {
        self.get_params().input_claim(accumulator)
    }
    fn compute_message(&mut self, round: usize, previous_claim: F) -> UniPoly<F>;
    fn ingest_challenge(&mut self, r_j: F::Challenge, round: usize);
    fn finalize(&mut self) {}
    fn cache_openings(
        &self,
        _accumulator: &mut ProverOpeningAccumulator<F>,
        _sumcheck_challenges: &[F::Challenge],
    ) {
    }
}

/// Verifier for one sumcheck instance.
pub trait SumcheckInstanceVerifier<F: SumcheckField> {
    fn get_params(&self) -> &dyn SumcheckInstanceParams<F>;
    fn degree(&self) -> usize {
        self.get_params().degree()
    }
    fn num_rounds(&self) -> usize {
        self.get_params().num_rounds()
    }
    fn round_offset(&self, max_num_rounds: usize) -> usize {
        max_num_rounds - self.num_rounds()
    }
    fn input_claim(&self, accumulator: &VerifierOpeningAccumulator<F>) -> F {
        self.get_params().input_claim(accumulator)
    }
    fn expected_output_claim(
        &self,
        _accumulator: &VerifierOpeningAccumulator<F>,
        sumcheck_challenges: &[F::Challenge],
    ) -> F;
    fn cache_openings(
        &self,
        _accumulator: &mut VerifierOpeningAccumulator<F>,
        _sumcheck_challenges: &[F::Challenge],
    ) {
    }
}
