// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod evaluation_domain;
pub mod fft;
pub mod lagrange;
pub mod polynomials;

use aptos_crypto::arkworks::random::less_insecure_random_point;
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::{CryptoRng, RngCore};

#[derive(CanonicalSerialize, CanonicalDeserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct GroupGenerators<E: Pairing> {
    pub g1: E::G1Affine,
    pub g2: E::G2Affine,
}

impl<E: Pairing> GroupGenerators<E> {
    pub fn sample<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            g1: less_insecure_random_point(rng),
            g2: less_insecure_random_point(rng),
        }
    }
}
