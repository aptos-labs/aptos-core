// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::range_proofs::dekart_univariate::{commit, setup, Commitment, PublicParameters};
use ark_ec::pairing::Pairing;
use ark_std::rand::{CryptoRng, RngCore};

pub fn range_proof_random_instance<E: Pairing, R: RngCore + CryptoRng>(
    n: usize,
    ell: usize,
    rng: &mut R,
) -> (
    PublicParameters<E>,
    Vec<E::ScalarField>,
    Commitment<E>,
    E::ScalarField,
) {
    let pp = setup(ell, n);

    let zz: Vec<E::ScalarField> = (0..n)
        .map(|_| {
            let val = rng.next_u64() >> (64 - ell);
            E::ScalarField::from(val)
        })
        .collect();

    let (cc, r) = commit(&pp, &zz, rng);
    (pp, zz, cc, r)
}
