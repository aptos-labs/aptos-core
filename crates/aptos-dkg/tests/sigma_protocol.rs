// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::{
    sigma_protocol::{
        self, homomorphism,
        homomorphism::{fixed_base_msms, fixed_base_msms::Trait, tuple::TupleHomomorphism},
    },
    Scalar,
};
use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup};
use ark_ff::UniformRand;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::thread_rng;

#[cfg(test)]
pub fn test_sigma_protocol<E, P>(instance: P, witness: P::Domain)
where
    E: Pairing,
    P: sigma_protocol::Trait<E>,
{
    let mut rng = thread_rng();

    let statement = instance.apply(&witness);

    let mut prover_transcript = merlin::Transcript::new(b"sigma-protocol-test");
    let proof = instance.prove(&witness, &mut prover_transcript, &mut rng);

    let mut verifier_transcript = merlin::Transcript::new(b"sigma-protocol-test");
    instance
        .verify(&statement, &proof, &mut verifier_transcript)
        .expect("Sigma protocol proof failed verification");
}

mod schnorr {
    use super::*;
    use ark_ec::VariableBaseMSM;
    use sigma_protocol::homomorphism::TrivialShape as CodomainShape;

    #[derive(Clone, Debug)]
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

    impl<E: Pairing> homomorphism::Trait for Schnorr<E> {
        type Codomain = CodomainShape<E::G1>;
        type Domain = Scalar<E>;

        fn apply(&self, input: &Self::Domain) -> Self::Codomain {
            self.apply_msm(self.msm_terms(input))
        }
    }

    impl<E: Pairing> fixed_base_msms::Trait for Schnorr<E> {
        type Base = E::G1Affine;
        type CodomainShape<T>
            = CodomainShape<T>
        where
            T: CanonicalSerialize + CanonicalDeserialize + Clone;
        type MsmInput = fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
        type MsmOutput = E::G1;
        type Scalar = E::ScalarField;

        fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
            CodomainShape(fixed_base_msms::MsmInput {
                bases: vec![self.g],
                scalars: vec![input.0],
            })
        }

        fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
            E::G1::msm(bases, scalars).expect("MSM failed in Schnorr")
        }
    }

    impl<E: Pairing> sigma_protocol::Trait<E> for Schnorr<E> {
        fn dst(&self) -> Vec<u8> {
            b"Schnorr".to_vec()
        }

        fn dst_verifier(&self) -> Vec<u8> {
            b"Schnorr-verifier".to_vec()
        }
    }
}

mod chaum_pedersen {
    use super::{schnorr::*, *};

    pub type ChaumPedersen<E> = TupleHomomorphism<Schnorr<E>, Schnorr<E>>;

    // Implementing e.g. Default here would require a wrapper, but then sigma_protocol::Trait would have to get re-implemented...
    pub fn make_chaum_pedersen_instance<E: Pairing>() -> ChaumPedersen<E> {
        let g1 = E::G1::generator().into_affine();
        let g2 = (g1 * E::ScalarField::from(123456789u64)).into_affine();

        let schnorr1 = Schnorr { g: g1 };
        let schnorr2 = Schnorr { g: g2 };

        TupleHomomorphism {
            hom1: schnorr1,
            hom2: schnorr2,
            dst: b"Chaum-Pedersen DST".to_vec(),
            dst_verifier: b"Chaum-Pedersen verifier DST".to_vec(),
        }
    }
}

#[test]
fn test_schnorr() {
    use schnorr::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = Scalar(<Bn254 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bn254, _>(Schnorr::default(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bls = Scalar(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bls12_381, _>(Schnorr::default(), witness_bls);
}

#[test]
fn test_chaum_pedersen() {
    use chaum_pedersen::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = Scalar(<Bn254 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bn254, _>(make_chaum_pedersen_instance(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bls = Scalar(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng));
    test_sigma_protocol::<Bls12_381, _>(make_chaum_pedersen_instance(), witness_bls);
}

#[test]
fn test_two_term_msm() {
    use aptos_dkg::{range_proofs::dekart_univariate_v2::two_term_msm::*, Scalar};

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = Witness {
        kzg_randomness: Scalar(<Bn254 as Pairing>::ScalarField::rand(&mut rng)),
        hiding_kzg_randomness: Scalar(<Bn254 as Pairing>::ScalarField::rand(&mut rng)),
    };
    test_sigma_protocol::<Bn254, _>(Homomorphism::default(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bn = Witness {
        kzg_randomness: Scalar(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng)),
        hiding_kzg_randomness: Scalar(<Bls12_381 as Pairing>::ScalarField::rand(&mut rng)),
    };
    test_sigma_protocol::<Bls12_381, _>(Homomorphism::default(), witness_bn);
}
