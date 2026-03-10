// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Merlin-backed transcript that implements `SumcheckTranscript`
//! so callers can drive sumcheck with an external Merlin transcript.

use crate::arkworks::sumcheck::{SumcheckField, SumcheckTranscript};
use ark_serialize::CanonicalSerialize;
use merlin::Transcript;

/// Wraps a Merlin transcript to implement `SumcheckTranscript` so sumcheck
/// (prove/verify) is bound to the same transcript as the parent protocol.
pub struct MerlinSumcheckTranscript<'a> {
    transcript: &'a mut Transcript,
}

impl<'a> MerlinSumcheckTranscript<'a> {
    pub fn new(transcript: &'a mut Transcript) -> Self {
        Self { transcript }
    }
}

impl<'a> SumcheckTranscript for MerlinSumcheckTranscript<'a> {
    fn append_scalar<F: SumcheckField + CanonicalSerialize>(
        &mut self,
        label: &'static [u8],
        scalar: &F,
    ) {
        // Keep the same fixed-width scalar encoding used by sumcheck transcripts.
        let mut buf = vec![0u8; 32];
        scalar
            .serialize_uncompressed(&mut buf)
            .expect("sumcheck scalar serialization");
        self.transcript.append_message(label, &buf);
    }

    fn append_scalars<F: SumcheckField + CanonicalSerialize>(
        &mut self,
        label: &'static [u8],
        scalars: &[F],
    ) {
        let mut buf = Vec::new();
        for s in scalars {
            s.serialize_uncompressed(&mut buf)
                .expect("sumcheck scalar serialization");
        }
        self.transcript.append_message(label, &buf);
    }

    fn challenge_scalar<F: SumcheckField + ark_ff::PrimeField>(&mut self) -> F {
        let mut buf = [0u8; 32];
        self.transcript
            .challenge_bytes(b"sumcheck_challenge", &mut buf);
        F::from_be_bytes_mod_order(&buf)
    }
}
