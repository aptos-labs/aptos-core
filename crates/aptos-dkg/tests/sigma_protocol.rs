// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::arkworks::{
    msm::MsmInput,
    random::{sample_field_element, sample_field_elements},
};
use aptos_dkg::{
    pvss::chunky::chunked_scalar_mul::Witness,
    sigma_protocol::{
        self, homomorphism,
        homomorphism::{
            fixed_base_msms,
            fixed_base_msms::Trait as _,
            tuple::{CurveGroupTupleHomomorphism, TupleHomomorphism},
            Trait as _,
        },
        Trait as _,
    },
    Scalar,
};
use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup};
use ark_ff::{fields::models::fp::MontBackend, Fp, FpConfig};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::thread_rng;
use std::fmt::Debug;

const CNTXT: &[u8; 32] = b"SIGMA-PROTOCOL-TESTS-SOK-CONTEXT";

pub fn test_sigma_protocol<H>(hom: H, witness: H::Domain)
where
    H: sigma_protocol::CurveGroupTrait,
{
    let mut rng = thread_rng();

    let statement = hom.apply(&witness);

    let (proof, normalized_statement) = hom.prove(&witness, statement, CNTXT, &mut rng);

    hom.verify(&normalized_statement, &proof, CNTXT, None, &mut rng)
        .expect("Sigma protocol proof failed verification");
}

// TODO: Find a way to make this more modular
fn test_imhomog_chaum_pedersen<
    E: Pairing<ScalarField = Fp<P, N>>,
    const N: usize,
    P: FpConfig<N>,
>(
    hom: chaum_pedersen::InhomogChaumPedersen<E>,
    witness: E::ScalarField,
) {
    let mut rng = thread_rng();

    let statement = hom.apply(&witness);

    let (proof, normalized_statement) = hom.prove(&witness, statement, CNTXT, &mut rng);

    hom.verify(&normalized_statement, &proof, CNTXT, None, &mut rng)
        .expect("Inhomogeneous Chaum-Pedersen sigma proof failed verification");
}

fn test_imhomog_scalar_mul<'a, E>(
    hom: chunked_scalar_mul::InhomogChunkedScalarMul<'a, E>,
    witness: Witness<E::ScalarField>,
) where
    E: Pairing,
{
    let mut rng = thread_rng();

    let statement = hom.apply(&witness);

    let (proof, normalized_statement) = hom.prove(&witness, statement, CNTXT, &mut rng);

    hom.verify(&normalized_statement, &proof, CNTXT, None, &mut rng)
        .expect("Inhomogeneous chunked scalar mul sigma proof failed verification");
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

    impl<C: CurveGroup<ScalarField = Fp<P, N>>, const N: usize, P: FpConfig<N>> homomorphism::Trait
        for Schnorr<C>
    {
        type Codomain = CodomainShape<C>;
        type CodomainNormalized = CodomainShape<C::Affine>;
        type Domain = Fp<P, N>;

        fn apply(&self, input: &Self::Domain) -> Self::Codomain {
            self.apply_msm(self.msm_terms(input))
        }

        fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
            <Schnorr<C> as fixed_base_msms::Trait>::normalize_output(value)
        }
    }

    impl<C: CurveGroup<ScalarField = Fp<P, N>>, const N: usize, P: FpConfig<N>>
        fixed_base_msms::Trait for Schnorr<C>
    {
        type Base = C::Affine;
        type CodomainShape<T>
            = CodomainShape<T>
        where
            T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
        type MsmOutput = C;
        type Scalar = C::ScalarField;

        fn msm_terms(
            &self,
            input: &Self::Domain,
        ) -> Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>> {
            CodomainShape(MsmInput {
                bases: vec![self.G],
                scalars: vec![*input],
            })
        }

        fn msm_eval(input: MsmInput<Self::Base, Self::Scalar>) -> Self::MsmOutput {
            // for the homomorphism we only need `input.bases()[0] * input.scalars()[0]`
            // but the verification needs a 3-term MSM... so we should really do a custom MSM which dispatches based on length TODO
            C::msm(input.bases(), input.scalars()).expect("MSM failed in Schnorr")
        }

        fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
            C::normalize_batch(&msm_output)
        }
    }

    impl<C: CurveGroup<ScalarField = Fp<P, N>>, const N: usize, P: FpConfig<N>>
        sigma_protocol::CurveGroupTrait for Schnorr<C>
    {
        type Group = C;

        fn dst(&self) -> Vec<u8> {
            b"SCHNORR_SIGMA_PROTOCOL_DST".to_vec()
        }
    }
}

mod chaum_pedersen {
    use super::{schnorr::*, *};

    pub type ChaumPedersen<C> = CurveGroupTupleHomomorphism<C, Schnorr<C>, Schnorr<C>>;

    // Implementing e.g. `Default` here would require a wrapper, but then `sigma_protocol::Trait` would have to get re-implemented...
    #[allow(non_snake_case)]
    pub fn make_chaum_pedersen_instance<
        C: CurveGroup<ScalarField = Fp<P, N>>,
        const N: usize,
        P: FpConfig<N>,
    >() -> ChaumPedersen<C> {
        let G_1 = C::generator().into_affine();
        let G_2 = (G_1 * C::ScalarField::from(123456789u64)).into_affine();

        let schnorr1 = Schnorr { G: G_1 };
        let schnorr2 = Schnorr { G: G_2 };

        CurveGroupTupleHomomorphism {
            hom1: schnorr1,
            hom2: schnorr2,
            _group: std::marker::PhantomData::<C>,
        }
    }

    pub type InhomogChaumPedersen<E> =
        TupleHomomorphism<Schnorr<<E as Pairing>::G1>, Schnorr<<E as Pairing>::G2>>;

    #[allow(non_snake_case)]
    pub fn make_inhomogeneous_chaum_pedersen_instance<
        E: Pairing<ScalarField = Fp<P, N>>,
        const N: usize,
        P: FpConfig<N>,
    >() -> InhomogChaumPedersen<E> {
        let G_1 = E::G1::generator().into_affine();
        let G_2 = E::G2::generator().into_affine();

        let schnorr1 = Schnorr { G: G_1 };
        let schnorr2 = Schnorr { G: G_2 };

        TupleHomomorphism {
            hom1: schnorr1,
            hom2: schnorr2,
        }
    }
}

mod chunked_scalar_mul {
    use super::*;
    use aptos_dkg::pvss::chunky::chunked_scalar_mul;
    use ark_ec::scalar_mul::BatchMulPreprocessing;

    pub type InhomogChunkedScalarMul<'a, E> = TupleHomomorphism<
        chunked_scalar_mul::Homomorphism<'a, <E as Pairing>::G1>,
        chunked_scalar_mul::Homomorphism<'a, <E as Pairing>::G2>,
    >;

    pub fn make_inhomogeneous_scalar_mul<'a, E: Pairing>(
        table1: &'a BatchMulPreprocessing<<E as Pairing>::G1>,
        table2: &'a BatchMulPreprocessing<<E as Pairing>::G2>,
    ) -> InhomogChunkedScalarMul<'a, E> {
        let g_1 = E::G1::generator().into_affine();
        let g_2 = E::G2::generator().into_affine();

        let hom1 = chunked_scalar_mul::Homomorphism {
            base: g_1,
            table: table1,
            ell: 16,
        };
        let hom2 = chunked_scalar_mul::Homomorphism {
            base: g_2,
            table: table2,
            ell: 16,
        };

        TupleHomomorphism { hom1, hom2 }
    }
}

#[test]
fn test_schnorr() {
    use schnorr::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = sample_field_element(&mut rng);
    test_sigma_protocol::<Schnorr<<Bn254 as Pairing>::G1>>(Schnorr::default(), witness_bn);

    // ---- Bls12_381 ----
    let witness_bls = sample_field_element(&mut rng);
    test_sigma_protocol::<Schnorr<<Bls12_381 as Pairing>::G1>>(Schnorr::default(), witness_bls);
}

#[test]
fn test_chaum_pedersen() {
    use chaum_pedersen::*;

    let mut rng = thread_rng();

    // ---- Bn254 ----
    let witness_bn = sample_field_element(&mut rng);
    test_sigma_protocol::<ChaumPedersen<<Bn254 as Pairing>::G1>>(
        make_chaum_pedersen_instance(),
        witness_bn,
    );
    let hom_bn = make_inhomogeneous_chaum_pedersen_instance::<
        Bn254,
        4,
        MontBackend<ark_bn254::FrConfig, 4>,
    >();
    test_imhomog_chaum_pedersen::<Bn254, 4, MontBackend<ark_bn254::FrConfig, 4>>(
        hom_bn, witness_bn,
    );

    // ---- Bls12_381 ----
    let witness_bls = sample_field_element(&mut rng);
    test_sigma_protocol::<ChaumPedersen<<Bls12_381 as Pairing>::G1>>(
        make_chaum_pedersen_instance(),
        witness_bls,
    );
    let hom_bls = make_inhomogeneous_chaum_pedersen_instance::<
        Bls12_381,
        4,
        MontBackend<ark_bls12_381::FrConfig, 4>,
    >();
    test_imhomog_chaum_pedersen::<Bls12_381, 4, MontBackend<ark_bls12_381::FrConfig, 4>>(
        hom_bls,
        witness_bls,
    );
}

#[test]
fn test_chunked_scalar_mul() {
    use aptos_dkg::pvss::chunky::{chunked_scalar_mul::Witness, chunks};
    use ark_bn254::Fr;
    use ark_ec::scalar_mul::BatchMulPreprocessing;
    use chunked_scalar_mul::make_inhomogeneous_scalar_mul;

    let mut rng = thread_rng();
    let ell = 16u8;

    let scalars = sample_field_elements(1, &mut rng);
    let chunked_values: Vec<Vec<Scalar<Fr>>> = scalars
        .iter()
        .map(|s| {
            chunks::scalar_to_le_chunks(ell, s)
                .into_iter()
                .map(Scalar)
                .collect::<Vec<_>>()
        })
        .collect();

    let witness = Witness {
        chunked_values: chunked_values.clone(),
    };

    let g_1 = <Bn254 as Pairing>::G1::generator().into_affine();
    let g_2 = <Bn254 as Pairing>::G2::generator().into_affine();
    let table1 = BatchMulPreprocessing::new(g_1.into(), 256);
    let table2 = BatchMulPreprocessing::new(g_2.into(), 256);

    let hom = make_inhomogeneous_scalar_mul::<Bn254>(&table1, &table2);
    test_imhomog_scalar_mul::<Bn254>(hom, witness);
}
