// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod evaluation_domain;
pub mod fft;
pub mod lagrange;
pub mod polynomials;

use ark_ec::{pairing::Pairing};
use ark_ff::UniformRand;
use ark_serialize::CanonicalSerialize;
use ark_std::rand::{CryptoRng, RngCore};

#[derive(CanonicalSerialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct GroupGenerators<E: Pairing> {
    pub g1: E::G1Affine,
    pub g2: E::G2Affine,
}

impl<E: Pairing> GroupGenerators<E> {
    /// Create a new GroupData with random G1 and G2 elements
    pub fn sample<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            g1: E::G1Affine::rand(rng),
            g2: E::G2Affine::rand(rng),
        }
    }
}