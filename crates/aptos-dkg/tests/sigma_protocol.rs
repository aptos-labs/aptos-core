// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::{
    algebra::{
        morphism,
        morphism::{DiagonalProductMorphism, Morphism},
    },
    sigma_protocol,
    sigma_protocol::SigmaProtocol,
};
use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup};
use ark_ff::UniformRand;
use ark_serialize::CanonicalSerialize;
use ark_std::rand::{thread_rng, CryptoRng, RngCore};

#[cfg(test)]
pub fn test_sigma_protocol<E, P>(instance: P, witness: P::Witness)
where
    E: Pairing,
    P: SigmaProtocol<E>,
{
    let mut rng = thread_rng();

    let hom = instance.homomorphism();
    let statement = hom.apply(&witness);

    let mut prover_transcript = merlin::Transcript::new(b"sigma-protocol-test");
    let proof = instance.prove(&witness, &mut prover_transcript, &mut rng);

    let mut verifier_transcript = merlin::Transcript::new(b"sigma-protocol-test");
    instance
        .verify(&statement, &proof, &mut verifier_transcript)
        .expect("Sigma protocol proof failed verification");
}

mod schnorr {
    use super::*;

    pub struct Schnorr<E: Pairing> {
        pub g: E::G1Affine,
    }

    impl<E: Pairing> Default for Schnorr<E> {
        fn default() -> Self {
            Self {
                g: E::G1::generator().into_affine(),
            }
        }
    }

    #[derive(CanonicalSerialize, Clone, Debug, PartialEq, Eq)]
    pub struct SchnorrDomain<E: Pairing>(pub E::ScalarField);
    impl<E: Pairing> sigma_protocol::Domain<E> for SchnorrDomain<E> {
        type Scalar = E::ScalarField;

        fn scaled_add(&self, other: &Self, c: E::ScalarField) -> Self {
            SchnorrDomain(self.0 + c * other.0)
        }

        fn sample_randomness<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
            SchnorrDomain(E::ScalarField::rand(rng))
        }
    }

    impl<E: Pairing> SigmaProtocol<E> for Schnorr<E> {
        type Hom = ExponentiateBase<E>;
        type Statement = E::G1;
        type Witness = SchnorrDomain<E>;

        const DST: &'static [u8] = b"Schnorr";
        const DST_VERIFIER: &'static [u8] = b"Schnorr-verifier";

        fn homomorphism(&self) -> Self::Hom {
            ExponentiateBase { g: self.g }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ExponentiateBase<E: Pairing> {
        pub g: E::G1Affine,
    }

    impl<E: Pairing> morphism::Morphism for ExponentiateBase<E> {
        type Codomain = E::G1;
        type Domain = SchnorrDomain<E>;

        fn apply(&self, input: &Self::Domain) -> Self::Codomain {
            self.g * input.0
        }
    }

    impl<E: Pairing> morphism::FixedBaseMSM for ExponentiateBase<E> {
        type Base = E::G1Affine;
        type Scalar = E::ScalarField;

        fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)> {
            vec![(vec![self.g], vec![input.0])]
        }

        fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base> {
            vec![output.into_affine()]
        }
    }
}

mod chaum_pedersen {
    use super::{schnorr::*, *};

    pub struct ChaumPedersen<E: Pairing> {
        pub g1: E::G1Affine,
        pub g2: E::G1Affine,
    }

    impl<E: Pairing> Default for ChaumPedersen<E> {
        fn default() -> Self {
            let g1 = E::G1::generator().into_affine();
            let g2 = (g1 * E::ScalarField::from(123456789u64)).into_affine();
            Self { g1, g2 }
        }
    }

    impl<E: Pairing> SigmaProtocol<E> for ChaumPedersen<E> {
        type Hom = DiagonalProductMorphism<ExponentiateBase<E>, ExponentiateBase<E>>;
        type Statement = (E::G1, E::G1);
        type Witness = SchnorrDomain<E>;

        const DST: &'static [u8] = b"Chaum-Pedersen";
        const DST_VERIFIER: &'static [u8] = b"Chaum-Pedersen-verifier";

        fn homomorphism(&self) -> Self::Hom {
            let h1 = ExponentiateBase { g: self.g1 };
            let h2 = ExponentiateBase { g: self.g2 };
            DiagonalProductMorphism {
                morphism1: h1,
                morphism2: h2,
            }
        }
    }
}

#[test]
fn test_schnorr() {
    use schnorr::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = SchnorrDomain(<Bn254 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bn254, _>(Schnorr::default(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bls = SchnorrDomain(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bls12_381, _>(Schnorr::default(), witness_bls);
}

#[test]
fn test_chaum_pedersen() {
    use chaum_pedersen::*;
    use schnorr::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = SchnorrDomain(<Bn254 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bn254, _>(ChaumPedersen::default(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bls = SchnorrDomain(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bls12_381, _>(ChaumPedersen::default(), witness_bls);
}
