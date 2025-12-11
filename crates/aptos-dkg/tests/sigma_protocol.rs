// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::arkworks::{
    msm::{IsMsmInput, MsmInput},
    random::sample_field_element,
};
use aptos_dkg::{
    sigma_protocol::{
        self, homomorphism,
        homomorphism::{
            fixed_base_msms,
            fixed_base_msms::Trait as _,
            tuple::{PairingTupleHomomorphism, TupleHomomorphism},
            Trait as _,
        },
    },
    Scalar,
};
use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::thread_rng;
use std::fmt::Debug;

#[cfg(test)]
pub fn test_sigma_protocol<C, H>(hom: H, witness: H::Domain)
where
    C: CurveGroup,
    H: sigma_protocol::Trait<C>,
{
    let mut rng = thread_rng();

    let statement = hom.apply(&witness);
    let ctxt = b"SIGMA-PROTOCOL-CONTEXT";

    let proof = hom.prove(&witness, &statement, ctxt, &mut rng);

    hom.verify(&statement, &proof, ctxt)
        .expect("Sigma protocol proof failed verification");
}

fn test_imhomog_chaum_pedersen<E>(
    hom: chaum_pedersen::InhomogChaumPedersen<E>,
    witness: Scalar<E::ScalarField>,
) where
    E: Pairing,
{
    let mut rng = thread_rng();

    let statement = hom.apply(&witness);
    let ctxt = b"SIGMA-PROTOCOL-CONTEXT";

    let proof = hom.prove(&witness, &statement, ctxt, &mut rng);

    hom.verify(&statement, &proof, ctxt)
        .expect("PairingTupleHomomorphism proof failed verification");
}

mod schnorr {
    use super::*;
    use sigma_protocol::homomorphism::TrivialShape as CodomainShape;

    #[allow(non_snake_case)]
    #[derive(CanonicalSerialize, Clone, Debug)]
    pub(crate) struct Schnorr<C: CurveGroup> {
        pub G: C::Affine,
    }

    // `C::Affine` doesn't seem to implement `Default`, otherwise it would've been derived for `Schnorr` here
    impl<C: CurveGroup> Default for Schnorr<C> {
        fn default() -> Self {
            Self {
                G: C::generator().into_affine(),
            }
        }
    }

    impl<C: CurveGroup> homomorphism::Trait for Schnorr<C> {
        type Codomain = CodomainShape<C>;
        type Domain = Scalar<C::ScalarField>;

        fn apply(&self, input: &Self::Domain) -> Self::Codomain {
            self.apply_msm(self.msm_terms(input))
        }
    }

    impl<C: CurveGroup> fixed_base_msms::Trait for Schnorr<C> {
        type CodomainShape<T>
            = CodomainShape<T>
        where
            T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
        type MsmInput = MsmInput<C::Affine, C::ScalarField>;
        type MsmOutput = C;
        type Scalar = C::ScalarField;

        fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
            CodomainShape(MsmInput {
                bases: vec![self.G],
                scalars: vec![input.0],
            })
        }

        fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
            C::msm(input.bases(), input.scalars()).expect("MSM failed in Schnorr")
        }
    }

    impl<C: CurveGroup> sigma_protocol::Trait<C> for Schnorr<C> {
        fn dst(&self) -> Vec<u8> {
            b"SCHNORR_SIGMA_PROTOCOL_DST".to_vec()
        }
    }
}

mod chaum_pedersen {
    use super::{schnorr::*, *};

    pub type ChaumPedersen<C> = TupleHomomorphism<Schnorr<C>, Schnorr<C>>;

    // Implementing e.g. `Default` here would require a wrapper, but then `sigma_protocol::Trait` would have to get re-implemented...
    #[allow(non_snake_case)]
    pub fn make_chaum_pedersen_instance<C: CurveGroup>() -> ChaumPedersen<C> {
        let G_1 = C::generator().into_affine();
        let G_2 = (G_1 * C::ScalarField::from(123456789u64)).into_affine();

        let schnorr1 = Schnorr { G: G_1 };
        let schnorr2 = Schnorr { G: G_2 };

        TupleHomomorphism {
            hom1: schnorr1,
            hom2: schnorr2,
        }
    }

    pub type InhomogChaumPedersen<E> =
        PairingTupleHomomorphism<E, Schnorr<<E as Pairing>::G1>, Schnorr<<E as Pairing>::G2>>;

    #[allow(non_snake_case)]
    pub fn make_inhomogeneous_chaum_pedersen_instance<E: Pairing>() -> InhomogChaumPedersen<E> {
        let G_1 = E::G1::generator().into_affine();
        let G_2 = E::G2::generator().into_affine();

        let schnorr1 = Schnorr { G: G_1 };
        let schnorr2 = Schnorr { G: G_2 };

        PairingTupleHomomorphism {
            hom1: schnorr1,
            hom2: schnorr2,
            _pairing: std::marker::PhantomData,
        }
    }
}

#[test]
fn test_schnorr() {
    use schnorr::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = Scalar(sample_field_element(&mut rng));
    test_sigma_protocol::<<Bn254 as Pairing>::G1, _>(Schnorr::default(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bls = Scalar(sample_field_element(&mut rng));
    test_sigma_protocol::<<Bls12_381 as Pairing>::G1, _>(Schnorr::default(), witness_bls);
}

#[test]
fn test_chaum_pedersen() {
    use chaum_pedersen::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = Scalar(sample_field_element(&mut rng));
    test_sigma_protocol::<<Bn254 as Pairing>::G1, _>(make_chaum_pedersen_instance(), witness_bn);
    test_imhomog_chaum_pedersen::<Bn254>(make_inhomogeneous_chaum_pedersen_instance(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bls = Scalar(sample_field_element(&mut rng));
    test_sigma_protocol::<<Bls12_381 as Pairing>::G1, _>(
        make_chaum_pedersen_instance(),
        witness_bls,
    );
    test_imhomog_chaum_pedersen::<Bls12_381>(
        make_inhomogeneous_chaum_pedersen_instance(),
        witness_bls,
    );
}
