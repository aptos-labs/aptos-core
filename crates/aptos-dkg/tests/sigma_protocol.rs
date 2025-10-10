// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::sigma_protocol::{
    self, homomorphism,
    homomorphism::{FixedBaseMsms, Trait, TupleHomomorphism},
};
use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup};
use ark_ff::UniformRand;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{thread_rng, CryptoRng, RngCore};

#[cfg(test)]
pub fn test_sigma_protocol<E, P>(instance: P, witness: P::Witness)
where
    E: Pairing,
    P: sigma_protocol::Trait<E>,
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

    pub(crate) struct Schnorr<E: Pairing> {
        pub g: E::G1Affine,
    }

    impl<E: Pairing> Default for Schnorr<E> {
        fn default() -> Self {
            Self {
                g: E::G1::generator().into_affine(),
            }
        }
    }

    #[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)] // TODO add SigmaProtocolWitness here
    pub(crate) struct Domain<E: Pairing>(pub(crate) E::ScalarField);
    impl<E: Pairing> sigma_protocol::Witness<E> for Domain<E> {
        type Scalar = E::ScalarField;

        fn scaled_add(self, other: &Self, c: E::ScalarField) -> Self {
            Domain(self.0 + c * other.0)
        }

        fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
            Domain(E::ScalarField::rand(rng))
        }
    }

    impl<E: Pairing> sigma_protocol::Trait<E> for Schnorr<E> {
        type Hom = ExponentiateBase<E>;
        type Statement = CodomainShape<E::G1>;
        type Witness = Domain<E>;

        const DST: &[u8] = b"Schnorr";
        const DST_VERIFIER: &[u8] = b"Schnorr-verifier";

        fn homomorphism(&self) -> Self::Hom {
            ExponentiateBase { g: self.g }
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) struct ExponentiateBase<E: Pairing> {
        pub g: E::G1Affine,
    }

    impl<E: Pairing> homomorphism::Trait for ExponentiateBase<E> {
        type Codomain = CodomainShape<E::G1>;
        type Domain = Domain<E>;

        fn apply(&self, input: &Self::Domain) -> Self::Codomain {
            self.apply_msm(self.msm_terms(input))
        }

        // fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        //     self.g * input.0
        // }
    }

    use ark_ec::VariableBaseMSM;
    use sigma_protocol::homomorphism::TrivialShape as CodomainShape;

    impl<E: Pairing> homomorphism::FixedBaseMsms for ExponentiateBase<E> {
        type Base = E::G1Affine;
        type CodomainShape<T>
            = CodomainShape<T>
        // ehhh?
        where
            T: CanonicalSerialize + CanonicalDeserialize + Clone + PartialEq + Eq;
        type MsmInput = homomorphism::MsmInput<Self::Base, Self::Scalar>;
        type MsmOutput = E::G1;
        type Scalar = E::ScalarField;

        fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
            // let mut scalars = Vec::with_capacity(2);
            // scalars.push(input.kzg_randomness.0);
            // scalars.push(input.hiding_kzg_randomness.0);

            // let mut bases = Vec::with_capacity(2);
            // bases.push(self.base_1);
            // bases.push(self.base_2);

            CodomainShape(homomorphism::MsmInput {
                bases: vec![self.g],
                scalars: vec![input.0],
            })
        }

        fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
            E::G1::msm(bases, scalars).expect("MSM failed in TwoTermMSM")
        }

        // fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)> {
        //     vec![(vec![self.g], vec![input.0])]
        // }

        // fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base> {
        //     vec![output.into_affine()]
        // }
    }
}

// mod chaum_pedersen {
//     use super::{schnorr::*, *};
//     use sigma_protocol::{homomorphism::TrivialShape as CodomainShape};
//     use crate::homomorphism::TupleCodomainShape;

//     pub(crate) struct ChaumPedersen<E: Pairing> {
//         pub g1: E::G1Affine,
//         pub g2: E::G1Affine,
//     }

//     impl<E: Pairing> Default for ChaumPedersen<E> {
//         fn default() -> Self {
//             let g1 = E::G1::generator().into_affine();
//             let g2 = (g1 * E::ScalarField::from(123456789u64)).into_affine();
//             Self { g1, g2 }
//         }
//     }

//     impl<E: Pairing> sigma_protocol::Trait<E> for ChaumPedersen<E> {
//         type Hom = TupleHomomorphism<ExponentiateBase<E>, ExponentiateBase<E>>;
//         type Statement = TupleCodomainShape<CodomainShape<E::G1>, CodomainShape<E::G1>>;
//         type Witness = Domain<E>;

//         const DST: &[u8] = b"Chaum-Pedersen";
//         const DST_VERIFIER: &[u8] = b"Chaum-Pedersen-verifier";

//         fn homomorphism(&self) -> Self::Hom {
//             let hom1 = ExponentiateBase { g: self.g1 };
//             let hom2 = ExponentiateBase { g: self.g2 };
//             TupleHomomorphism { hom1, hom2 }
//         }
//     }
// }

#[test]
fn test_schnorr() {
    use schnorr::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = Domain(<Bn254 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bn254, _>(Schnorr::default(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bls = Domain(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bls12_381, _>(Schnorr::default(), witness_bls);
}

// #[test]
// fn test_chaum_pedersen() {
//     use chaum_pedersen::*;
//     use schnorr::*;

//     let mut rng = thread_rng();

//     // ---- Bn254 ----
//     let witness_bn = Domain(<Bn254 as Pairing>::ScalarField::rand(&mut rng));
//     test_sigma_protocol::<Bn254, _>(ChaumPedersen::default(), witness_bn);

//     // ---- Bls12_381 ----
//     let witness_bls = Domain(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng));
//     test_sigma_protocol::<Bls12_381, _>(ChaumPedersen::default(), witness_bls);
// }

#[test]
fn test_dekart_sigma() {
    use aptos_dkg::range_proofs::dekart_univariate_v2::two_term_msm::*;
    use aptos_dkg::Scalar;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = Witness{kzg_randomness: Scalar(<Bn254 as Pairing>::ScalarField::rand(&mut rng)), hiding_kzg_randomness: Scalar(<Bn254 as Pairing>::ScalarField::rand(&mut rng))};
    test_sigma_protocol::<Bn254, _>(SigmaProtocol::default(), witness_bn);

    // // ---- Bls12_381 ----
    // let witness_bls = Domain(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng));
    // test_sigma_protocol::<Bls12_381, _>(Schnorr::default(), witness_bls);
}