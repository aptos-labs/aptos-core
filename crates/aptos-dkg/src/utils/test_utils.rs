// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::range_proofs::traits::BatchedRangeProof;
use ark_ec::pairing::Pairing;
use ark_std::rand::{CryptoRng, RngCore};

/// A reusable setup structure.
pub struct RangeProofSetup<E: Pairing, B: BatchedRangeProof<E>> {
    pub pk: B::ProverKey,
    pub vk: B::VerificationKey,
    pub values: Vec<B::Input>,
    pub comm: B::Commitment,
    pub r: B::CommitmentRandomness,
}

pub fn range_proof_random_instance<E: Pairing, B: BatchedRangeProof<E>, R: RngCore + CryptoRng>(
    n: usize,
    ell: usize,
    rng: &mut R,
) -> RangeProofSetup<E, B> {
    let (pk, vk) = B::setup(n, ell, rng); // TODO: potentially change these values back to n + 10 and ell + 10?

    let ell_bit_values: Vec<B::Input> = (0..n)
        .map(|_| {
            let val = rng.next_u64() >> (64 - ell);
            B::Input::from(val)
        })
        .collect();

    let (comm, r) = B::commit(
        &B::commitment_key_from_prover_key(&pk),
        &ell_bit_values,
        rng,
    );
    RangeProofSetup{pk, vk, values: ell_bit_values, comm, r}
}
