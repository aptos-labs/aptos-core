use std::ops::Mul;
use ark_ec::Group;
use ark_std::UniformRand;
use rand::{CryptoRng, Rng};
use crate::elgamal::ElGamalFriendlyGroup;

pub struct Bls12381G1 {}

impl ElGamalFriendlyGroup for Bls12381G1 {
    type Scalar = ark_bls12_381::Fr;
    type Element = ark_bls12_381::G1Projective;

    fn rand_scalar<R: CryptoRng + RngCore>(rng: &mut R) -> Self::Scalar {
        Self::Scalar::rand(rng)
    }

    fn generator_mul(scalar: &Self::Scalar) -> Self::Element {
        Self::Element::generator().mul(scalar)
    }

    fn add(a: &Self::Element, b: &Self::Element) -> Self::Element {
        a + b
    }

    fn sub(a: &Self::Element, b: &Self::Element) -> Self::Element {
        a - b
    }

    fn mul(a: &Self::Element, s: &Self::Scalar) -> Self::Element {
        a.mul(s)
    }
}
