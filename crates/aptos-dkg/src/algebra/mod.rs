// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod evaluation_domain;
pub mod fft;
pub mod lagrange;
pub mod polynomials;

use ark_ec::pairing::Pairing;
use ark_ff::UniformRand;
use ark_serialize::CanonicalSerialize;
use ark_std::rand::{CryptoRng, RngCore};

#[derive(CanonicalSerialize, Debug, Clone, PartialEq, Eq)]
pub struct GroupData<E: Pairing> {
    pub g1: E::G1Affine, // TODO: could also name these one_1 and one_2?
    pub g2: E::G2Affine,
}

impl<E: Pairing> GroupData<E> {
    /// Create a new GroupData with random G1 and G2 elements
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            g1: E::G1Affine::rand(rng),
            g2: E::G2Affine::rand(rng),
        }
    }
}
