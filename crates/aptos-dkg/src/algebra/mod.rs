// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod evaluation_domain;
pub mod fft;
pub mod lagrange;
pub mod polynomials;

use ark_ec::{pairing::Pairing, AffineRepr};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct GroupGenerators<E: Pairing> {
    pub g1: E::G1Affine,
    pub g2: E::G2Affine,
}

impl<E: Pairing> Default for GroupGenerators<E> {
    fn default() -> Self {
        Self {
            g1: E::G1Affine::generator(),
            g2: E::G2Affine::generator(),
        }
    }
}
