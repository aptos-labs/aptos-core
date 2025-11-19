// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::range_proofs::traits::BatchedRangeProof;
use ark_ec::pairing::Pairing;
use rand::{CryptoRng, RngCore};

/// Generate a **random instance** (values + commitment) given a fixed setup.
pub fn range_proof_random_instance<E: Pairing, B: BatchedRangeProof<E>, R: RngCore + CryptoRng>(
    pk: &B::ProverKey,
    n: usize,
    ell: usize,
    rng: &mut R,
) -> (Vec<B::Input>, B::Commitment, B::CommitmentRandomness) {
    // TODO: One might want to assert something like n <= pk.max_n here, for which you'd have to e.g. add a trait HasMaxN to ProverKey
    let ell_bit_values: Vec<B::Input> = (0..n)
        .map(|_| {
            let val = rng.next_u64() >> (64 - ell);
            B::Input::from(val)
        })
        .collect();

    let (comm, r) = B::commit(&B::commitment_key_from_prover_key(pk), &ell_bit_values, rng);

    (ell_bit_values, comm, r)
}
