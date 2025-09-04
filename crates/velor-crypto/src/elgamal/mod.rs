// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides ElGamal generic constructions and concrete schemes.

use rand_core::{CryptoRng, RngCore};
use std::fmt::Debug;

/// This trait captures the group operations needed to implement ElGamal.
pub trait ElGamalFriendlyGroup {
    /// The scalar type.
    type Scalar: Debug;

    /// The group element type.
    type Element: Debug + Eq;

    /// Generate a random scalar.
    fn rand_scalar<R: CryptoRng + RngCore>(rng: &mut R) -> Self::Scalar;

    /// Compute `s*G` where `s` is a scalar and `G` is the group generator.
    fn generator_mul(scalar: &Self::Scalar) -> Self::Element;

    /// Compute `A+B` where `A` and `B` are 2 group elements.
    fn add(a: &Self::Element, b: &Self::Element) -> Self::Element;

    /// Compute `A-B` where `A` and `B` are 2 group elements.
    fn sub(a: &Self::Element, b: &Self::Element) -> Self::Element;

    /// Compute `s*A` where `A` is a group element and `s` is a scalar.
    fn mul(a: &Self::Element, s: &Self::Scalar) -> Self::Element;

    /// Generate a random group element.
    fn rand_element<R: CryptoRng + RngCore>(rng: &mut R) -> Self::Element {
        Self::generator_mul(&Self::rand_scalar(rng))
    }
}

/// ElGamal encryption scheme over Curve25519.
pub mod curve25519;

/// Return a key pair  `(private_key, public_key)` for El Gamal encryption over BLS12-381 G1.
pub fn key_gen<G: ElGamalFriendlyGroup, R: CryptoRng + RngCore>(
    rng: &mut R,
) -> (G::Scalar, G::Element) {
    let sk = G::rand_scalar(rng);
    let pk = G::generator_mul(&sk);
    (sk, pk)
}

/// ElGamal encryption.
pub fn encrypt<G: ElGamalFriendlyGroup, R: CryptoRng + RngCore>(
    rng: &mut R,
    pk: &G::Element,
    msg: &G::Element,
) -> (G::Element, G::Element) {
    let r = G::rand_scalar(rng);
    let c0 = G::generator_mul(&r);
    let c1 = G::add(msg, &G::mul(pk, &r));
    (c0, c1)
}

/// ElGamal decryption.
pub fn decrypt<G: ElGamalFriendlyGroup>(
    sk: &G::Scalar,
    c0: &G::Element,
    c1: &G::Element,
) -> G::Element {
    G::sub(c1, &G::mul(c0, sk))
}

#[cfg(test)]
fn test_keygen_enc_dec<G: ElGamalFriendlyGroup>() {
    let mut rng = rand_core::OsRng;
    let (sk, pk) = key_gen::<G, _>(&mut rng);
    let msg = G::rand_element(&mut rng);
    let (c0, c1) = encrypt::<G, _>(&mut rng, &pk, &msg);
    assert_eq!(msg, decrypt::<G>(&sk, &c0, &c1));
}
