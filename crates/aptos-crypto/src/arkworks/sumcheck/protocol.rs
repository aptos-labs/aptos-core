// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Batched sumcheck prove/verify (from Jolt, no jolt dep).

use crate::arkworks::sumcheck::{
    field::SumcheckField,
    opening::{ProverOpeningAccumulator, VerifierOpeningAccumulator},
    traits::{SumcheckInstanceProver, SumcheckInstanceVerifier},
    transcript::SumcheckTranscript,
    unipoly::{CompressedUniPoly, UniPoly},
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

#[derive(Debug, Clone)]
pub enum SumcheckError {
    InvalidInputLength(usize, usize),
    VerificationFailed,
}

/// Clear (non-ZK) sumcheck proof.
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ClearSumcheckProof<F: SumcheckField> {
    pub compressed_polys: Vec<CompressedUniPoly<F>>,
}

impl<F: SumcheckField + ark_ff::PrimeField> ClearSumcheckProof<F> {
    pub fn new(compressed_polys: Vec<CompressedUniPoly<F>>) -> Self {
        Self { compressed_polys }
    }

    pub fn verify<T: SumcheckTranscript>(
        &self,
        claim: F,
        num_rounds: usize,
        degree_bound: usize,
        transcript: &mut T,
    ) -> Result<(F, Vec<F::Challenge>), SumcheckError> {
        let mut e = claim;
        let mut r: Vec<F::Challenge> = Vec::new();

        if self.compressed_polys.len() != num_rounds {
            return Err(SumcheckError::InvalidInputLength(
                num_rounds,
                self.compressed_polys.len(),
            ));
        }
        for poly in &self.compressed_polys {
            if poly.degree() > degree_bound {
                return Err(SumcheckError::InvalidInputLength(
                    degree_bound,
                    poly.degree(),
                ));
            }
            transcript.append_scalars(b"sumcheck_poly", &poly.coeffs_except_linear_term);
            let r_i = transcript.challenge_scalar::<F>();
            let r_chal = F::into_challenge(r_i);
            r.push(r_chal);
            e = poly.eval_from_hint(&e, &r_chal);
        }
        Ok((e, r))
    }
}

pub enum BatchedSumcheck {}

impl BatchedSumcheck {
    /// Prove with one or more sumcheck instances. Returns (proof, challenges, initial_batched_claim).
    pub fn prove<
        F: SumcheckField + CanonicalSerialize + ark_ff::PrimeField,
        T: SumcheckTranscript,
    >(
        mut sumcheck_instances: Vec<&mut dyn SumcheckInstanceProver<F>>,
        opening_accumulator: &mut ProverOpeningAccumulator<F>,
        transcript: &mut T,
    ) -> (ClearSumcheckProof<F>, Vec<F::Challenge>, F) {
        let max_num_rounds = sumcheck_instances
            .iter()
            .map(|s| s.num_rounds())
            .max()
            .unwrap();

        for sumcheck in sumcheck_instances.iter() {
            let input_claim = sumcheck.input_claim(opening_accumulator);
            transcript.append_scalar(b"sumcheck_claim", &input_claim);
        }
        let batching_coeffs: Vec<F> = transcript.challenge_vector(sumcheck_instances.len());

        let mut individual_claims: Vec<F> = sumcheck_instances
            .iter()
            .map(|s| {
                let input_claim = s.input_claim(opening_accumulator);
                input_claim.mul_pow_2(max_num_rounds - s.num_rounds())
            })
            .collect();

        let initial_batched_claim: F = individual_claims
            .iter()
            .zip(batching_coeffs.iter())
            .map(|(c, b)| *c * *b)
            .sum();

        let mut r_sumcheck: Vec<F::Challenge> = Vec::with_capacity(max_num_rounds);
        let mut compressed_polys: Vec<CompressedUniPoly<F>> = Vec::with_capacity(max_num_rounds);
        let two_inv = F::from(2u64).inverse().unwrap();

        for round in 0..max_num_rounds {
            let univariate_polys: Vec<UniPoly<F>> = sumcheck_instances
                .iter_mut()
                .zip(individual_claims.iter())
                .map(|(sumcheck, previous_claim)| {
                    let num_rounds = sumcheck.num_rounds();
                    let offset = sumcheck.round_offset(max_num_rounds);
                    let active = round >= offset && round < offset + num_rounds;
                    if active {
                        sumcheck.compute_message(round - offset, *previous_claim)
                    } else {
                        UniPoly::from_coeff(vec![*previous_claim * two_inv])
                    }
                })
                .collect();

            let batched_poly: UniPoly<F> = univariate_polys
                .iter()
                .zip(batching_coeffs.iter())
                .fold(UniPoly::zero(), |mut acc, (poly, coeff)| {
                    acc += &(poly * *coeff);
                    acc
                });

            let compressed_poly = batched_poly.compress();
            transcript.append_scalars(b"sumcheck_poly", &compressed_poly.coeffs_except_linear_term);
            let r_j = transcript.challenge_scalar::<F>();
            let r_chal = F::into_challenge(r_j);
            r_sumcheck.push(r_chal);

            for (claim, poly) in individual_claims.iter_mut().zip(univariate_polys) {
                *claim = poly.evaluate(&r_chal);
            }
            for sumcheck in sumcheck_instances.iter_mut() {
                let num_rounds = sumcheck.num_rounds();
                let offset = sumcheck.round_offset(max_num_rounds);
                if round >= offset && round < offset + num_rounds {
                    sumcheck.ingest_challenge(r_chal, round - offset);
                }
            }
            compressed_polys.push(compressed_poly);
        }

        for sumcheck in sumcheck_instances.iter_mut() {
            sumcheck.finalize();
        }
        for sumcheck in sumcheck_instances.iter() {
            let offset = sumcheck.round_offset(max_num_rounds);
            let r_slice = &r_sumcheck[offset..offset + sumcheck.num_rounds()];
            sumcheck.cache_openings(opening_accumulator, r_slice);
        }
        opening_accumulator.flush_to_transcript(transcript);

        (
            ClearSumcheckProof::new(compressed_polys),
            r_sumcheck,
            initial_batched_claim,
        )
    }

    /// Verify a standard (non-ZK) sumcheck proof.
    pub fn verify_standard<
        F: SumcheckField + CanonicalSerialize + ark_ff::PrimeField,
        T: SumcheckTranscript,
    >(
        proof: &ClearSumcheckProof<F>,
        sumcheck_instances: Vec<&dyn SumcheckInstanceVerifier<F>>,
        opening_accumulator: &mut VerifierOpeningAccumulator<F>,
        transcript: &mut T,
    ) -> Result<Vec<F::Challenge>, SumcheckError> {
        let max_degree = sumcheck_instances.iter().map(|s| s.degree()).max().unwrap();
        let max_num_rounds = sumcheck_instances
            .iter()
            .map(|s| s.num_rounds())
            .max()
            .unwrap();

        for sumcheck in sumcheck_instances.iter() {
            let input_claim = sumcheck.input_claim(opening_accumulator);
            transcript.append_scalar(b"sumcheck_claim", &input_claim);
        }
        let batching_coeffs: Vec<F> = transcript.challenge_vector(sumcheck_instances.len());

        let claim: F = sumcheck_instances
            .iter()
            .zip(batching_coeffs.iter())
            .map(|(s, coeff)| {
                let input_claim = s.input_claim(opening_accumulator);
                input_claim.mul_pow_2(max_num_rounds - s.num_rounds()) * *coeff
            })
            .sum();

        let (output_claim, r_sumcheck) =
            proof.verify(claim, max_num_rounds, max_degree, transcript)?;

        let expected_output_claim: F = sumcheck_instances
            .iter()
            .zip(batching_coeffs.iter())
            .map(|(s, coeff)| {
                let r_slice = &r_sumcheck[max_num_rounds - s.num_rounds()..];
                s.cache_openings(opening_accumulator, r_slice);
                let c = s.expected_output_claim(opening_accumulator, r_slice);
                c * *coeff
            })
            .sum();

        opening_accumulator.flush_to_transcript(transcript);

        if output_claim != expected_output_claim {
            return Err(SumcheckError::VerificationFailed);
        }
        Ok(r_sumcheck)
    }

    /// Same as verify_standard but runs all rounds and pushes a diagnostic message for each
    /// round and the final check, then returns Err with diagnostics if verification fails.
    /// Use this to print all check results instead of stopping at the first failure.
    pub fn verify_standard_with_diagnostics<
        F: SumcheckField + CanonicalSerialize + ark_ff::PrimeField,
        T: SumcheckTranscript,
    >(
        proof: &ClearSumcheckProof<F>,
        sumcheck_instances: Vec<&dyn SumcheckInstanceVerifier<F>>,
        opening_accumulator: &mut VerifierOpeningAccumulator<F>,
        transcript: &mut T,
        diagnostics: &mut Vec<String>,
    ) -> Result<Vec<F::Challenge>, SumcheckError> {
        let max_degree = sumcheck_instances.iter().map(|s| s.degree()).max().unwrap();
        let max_num_rounds = sumcheck_instances
            .iter()
            .map(|s| s.num_rounds())
            .max()
            .unwrap();

        for sumcheck in sumcheck_instances.iter() {
            let input_claim = sumcheck.input_claim(opening_accumulator);
            transcript.append_scalar(b"sumcheck_claim", &input_claim);
        }
        let batching_coeffs: Vec<F> = transcript.challenge_vector(sumcheck_instances.len());

        let claim: F = sumcheck_instances
            .iter()
            .zip(batching_coeffs.iter())
            .map(|(s, coeff)| {
                let input_claim = s.input_claim(opening_accumulator);
                input_claim.mul_pow_2(max_num_rounds - s.num_rounds()) * *coeff
            })
            .sum();

        diagnostics.push(format!("sumcheck claim (batched): {:?}", claim));

        let (output_claim, r_sumcheck) =
            match proof.verify(claim, max_num_rounds, max_degree, transcript) {
                Ok(t) => t,
                Err(e) => {
                    diagnostics.push(format!("sumcheck proof.verify failed: {:?}", e));
                    return Err(e);
                },
            };

        let expected_output_claim: F = sumcheck_instances
            .iter()
            .zip(batching_coeffs.iter())
            .map(|(s, coeff)| {
                let r_slice = &r_sumcheck[max_num_rounds - s.num_rounds()..];
                s.cache_openings(opening_accumulator, r_slice);
                let c = s.expected_output_claim(opening_accumulator, r_slice);
                c * *coeff
            })
            .sum();

        opening_accumulator.flush_to_transcript(transcript);

        let final_ok = output_claim == expected_output_claim;
        diagnostics.push(format!(
            "sumcheck final: output_claim={:?}, expected_output_claim={:?}, match={}",
            output_claim, expected_output_claim, final_ok
        ));
        if !final_ok {
            return Err(SumcheckError::VerificationFailed);
        }
        Ok(r_sumcheck)
    }
}
