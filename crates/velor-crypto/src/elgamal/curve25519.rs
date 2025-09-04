// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::elgamal::ElGamalFriendlyGroup;
use rand_core::{CryptoRng, RngCore};
use std::ops::Mul;

/// ElGamal encryption over Curve25519.
pub struct Curve25519 {}

impl ElGamalFriendlyGroup for Curve25519 {
    type Element = curve25519_dalek::edwards::EdwardsPoint;
    type Scalar = curve25519_dalek::scalar::Scalar;

    fn rand_scalar<R: CryptoRng + RngCore>(rng: &mut R) -> Self::Scalar {
        Self::Scalar::random(rng)
    }

    fn generator_mul(scalar: &Self::Scalar) -> Self::Element {
        curve25519_dalek::constants::ED25519_BASEPOINT_TABLE.mul(scalar)
    }

    fn add(a: &Self::Element, b: &Self::Element) -> Self::Element {
        a + b
    }

    fn sub(a: &Self::Element, b: &Self::Element) -> Self::Element {
        a - b
    }

    fn mul(a: &Self::Element, s: &Self::Scalar) -> Self::Element {
        s * a
    }
}

#[cfg(test)]
mod tests {
    use crate::elgamal::{curve25519::Curve25519, test_keygen_enc_dec};

    #[test]
    fn basic() {
        test_keygen_enc_dec::<Curve25519>()
    }
}
