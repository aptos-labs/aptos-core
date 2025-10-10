// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::range_proofs::traits::BatchedRangeProof;
use ark_ec::pairing::Pairing;
use ark_std::rand::{CryptoRng, RngCore};

pub fn range_proof_random_instance<E: Pairing, B: BatchedRangeProof<E>, R: RngCore + CryptoRng>(
    n: usize,
    ell: usize,
    rng: &mut R,
) -> (
    B::ProverKey,
    B::VerificationKey,
    Vec<B::Input>,
    B::Commitment,
    B::CommitmentRandomness,
) {
    let (pk, vk) = B::setup(n + 0, ell + 0, rng); // TODO: change these values?

    let ell_bit_values: Vec<B::Input> = (0..n)
        .map(|_| {
            let val = rng.next_u64() >> (64 - ell);
            B::Input::from(val)
        })
        .collect();

    let (comm, r) = B::commit(&B::commitment_key_from_prover_key(&pk), &ell_bit_values, rng);
    (pk, vk, ell_bit_values, comm, r)
}
