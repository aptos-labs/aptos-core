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
    let (pk, vk) = B::setup(n + 10, ell + 10, rng); // TODO: change these values?

    let zz: Vec<B::Input> = (0..n)
        .map(|_| {
            let val = rng.next_u64() >> (64 - ell);
            B::Input::from(val)
        })
        .collect();

    let (cc, r) = B::commit(&pk, &zz, rng);
    (pk, vk, zz, cc, r)
}
