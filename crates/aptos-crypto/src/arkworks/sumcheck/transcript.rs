// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Transcript trait for sumcheck Fiat-Shamir.

use crate::arkworks::sumcheck::field::SumcheckField;
use ark_serialize::CanonicalSerialize;

/// Transcript interface for sumcheck so that callers (e.g. range proofs) can drive
/// the sumcheck with their own transcript (e.g. Merlin).
pub trait SumcheckTranscript {
    fn append_scalar<F: SumcheckField + CanonicalSerialize>(
        &mut self,
        label: &'static [u8],
        scalar: &F,
    );
    fn append_scalars<F: SumcheckField + CanonicalSerialize>(
        &mut self,
        label: &'static [u8],
        scalars: &[F],
    );
    fn challenge_scalar<F: SumcheckField + ark_ff::PrimeField>(&mut self) -> F;
    fn challenge_vector<F: SumcheckField + ark_ff::PrimeField>(&mut self, len: usize) -> Vec<F> {
        (0..len).map(|_| self.challenge_scalar::<F>()).collect()
    }
}
