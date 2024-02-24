// Copyright © Aptos Foundation

use rand_core::{CryptoRng, RngCore};
use std::fmt::Debug;

pub trait ElGamalFriendlyGroup {
    type Scalar: Debug;
    type Element: Debug + Eq;

    fn rand_scalar<R: CryptoRng + RngCore>(rng: &mut R) -> Self::Scalar;
    fn generator_mul(scalar: &Self::Scalar) -> Self::Element;
    fn add(a: &Self::Element, b: &Self::Element) -> Self::Element;
    fn sub(a: &Self::Element, b: &Self::Element) -> Self::Element;
    fn mul(a: &Self::Element, s: &Self::Scalar) -> Self::Element;

    fn rand_element<R: CryptoRng + RngCore>(rng: &mut R) -> Self::Element {
        Self::generator_mul(&Self::rand_scalar(rng))
    }
}

pub mod curve25519;
pub mod bls12381g1;

/// Return a key pair  `(private_key, public_key)` for El Gamal encryption over BLS12-381 G1.
pub fn key_gen<G: ElGamalFriendlyGroup, R: CryptoRng + RngCore>(
    rng: &mut R,
) -> (G::Scalar, G::Element) {
    let sk = G::rand_scalar(rng);
    let pk = G::generator_mul(&sk);
    (sk, pk)
}

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
