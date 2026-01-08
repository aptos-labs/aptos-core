// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    algebra::polynomials,
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, fixed_base_msms::Trait as FixedBaseMsmsTrait, Trait},
    },
    Scalar,
};
use anyhow::ensure;
#[allow(unused_imports)] // This is used but due to some bug it is not noticed by the compiler
use aptos_crypto::arkworks::random::UniformRand;
use aptos_crypto::{
    arkworks::{
        msm::{IsMsmInput, MsmInput},
        random::{sample_field_element, unsafe_random_point},
        GroupGenerators,
    },
};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    AdditiveGroup, CurveGroup, VariableBaseMSM,
};
use ark_ff::{Field, PrimeField};
use ark_poly::{
    polynomial::univariate::DensePolynomial, univariate::DenseOrSparsePolynomial, EvaluationDomain,
};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize
};
use rand::{CryptoRng, RngCore};
use sigma_protocol::homomorphism::TrivialShape as CodomainShape;
use std::{borrow::Cow, fmt::Debug};
use aptos_crypto::utils::assert_power_of_two;
pub use srs::*;

pub type Commitment<E> = CodomainShape<<E as Pairing>::G1>;

pub type CommitmentRandomness<F> = Scalar<F>;

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct OpeningProof<E: Pairing> {
    pub(crate) pi_1: Commitment<E>,
    pub(crate) pi_2: E::G1,
}

impl<E: Pairing> OpeningProof<E> {
    /// Generates a random looking opening proof (but not a valid one).
    /// Useful for testing and benchmarking. TODO: might be able to derive this through macros etc
    pub fn generate<R: rand::Rng + rand::CryptoRng>(rng: &mut R) -> Self {
        Self {
            pi_1: sigma_protocol::homomorphism::TrivialShape(
                unsafe_random_point::<E::G1Affine, _>(rng).into(),
            ),
            pi_2: unsafe_random_point::<E::G1Affine, _>(rng).into(),
        }
    }
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerificationKey<E: Pairing> {
    pub xi_2: E::G2Affine,
    pub tau_2: E::G2Affine,
    pub group_generators: GroupGenerators<E>,
}

// For Zeromorph one also need powers of tau in g2
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct VerificationKeyExtra<E: Pairing> {
    pub vk: VerificationKey<E>,
    pub g2_powers: Vec<E::G2Affine>,
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommitmentKey<E: Pairing> {
    pub xi_1: E::G1Affine,
    pub tau_1: E::G1Affine,
    pub msm_basis: SrsBasis<E::G1Affine>,
    pub eval_dom: ark_poly::Radix2EvaluationDomain<E::ScalarField>,
    pub roots_of_unity_in_eval_dom: Vec<E::ScalarField>,
    pub g1: E::G1Affine,
    pub m_inv: E::ScalarField,
}

#[derive(CanonicalSerialize, Debug, Clone)]
pub struct Trapdoor<E: Pairing> {
    pub xi: E::ScalarField,
    pub tau: E::ScalarField,
}

impl<E: Pairing> Trapdoor<E> {
    pub fn rand<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            xi: sample_field_element(rng),
            tau: sample_field_element(rng),
        }
    }
}

pub fn setup<E: Pairing>(
    m: usize,
    basis_type: SrsType,
    group_generators: GroupGenerators<E>,
    trapdoor: Trapdoor<E>,
) -> (
    VerificationKey<E>,
    CommitmentKey<E>,
) {
    assert_power_of_two(m);

    let GroupGenerators { g1, g2 } = group_generators;
    let Trapdoor { xi, tau } = trapdoor;

    let (xi_1, tau_1) = ((g1 * xi).into_affine(), (g1 * tau).into_affine());
    let (xi_2, tau_2) = ((g2 * xi).into_affine(), (g2 * tau).into_affine());

    let eval_dom =
        ark_poly::Radix2EvaluationDomain::<E::ScalarField>::new(m)
            .expect("Could not construct evaluation domain");

    let msm_basis = match basis_type {
        SrsType::Lagrange => SrsBasis::Lagrange {
            lagr_g1: lagrange_basis::<E::G1>(m, g1, eval_dom, tau),
        },
        SrsType::PowersOfTau => SrsBasis::PowersOfTau {
            tau_powers_g1: powers_of_tau::<E::G1Affine>(g1, tau, m),
        },
    };

    let roots_of_unity_in_eval_dom = eval_dom.elements().collect();
    let m_inv = E::ScalarField::from(m as u64).inverse().unwrap();

    (
        VerificationKey {
            xi_2,
            tau_2,
            group_generators,
        },
        CommitmentKey {
            xi_1,
            tau_1,
            msm_basis,
            eval_dom,
            roots_of_unity_in_eval_dom,
            g1,
            m_inv,
        },
    )
}

pub fn setup_extra<E: Pairing>(
    m: usize,
    basis_type: SrsType,
    group_generators: GroupGenerators<E>,
    trapdoor: Trapdoor<E>,
) -> (VerificationKeyExtra<E>, CommitmentKey<E>) {
    let tau = trapdoor.tau;

    let (vk, ck) = setup(m, basis_type, group_generators, trapdoor);

    let g2_powers = powers_of_tau::<E::G2Affine>(
        vk.group_generators.g2,
        tau,
        m,
    );

    (
        VerificationKeyExtra { vk, g2_powers },
        ck,
    )
}

pub fn commit_with_randomness<E: Pairing>(
    ck: &CommitmentKey<E>,
    values: &[E::ScalarField],
    r: &CommitmentRandomness<E::ScalarField>,
) -> Commitment<E> {
    commit_with_randomness_and_offset(ck, values, r, 0)
}

pub fn commit_with_randomness_and_offset<E: Pairing>(
    ck: &CommitmentKey<E>,
    values: &[E::ScalarField],
    r: &CommitmentRandomness<E::ScalarField>,
    offset: usize,
) -> Commitment<E> {
    let msm_basis: &[E::G1Affine] = match &ck.msm_basis {
        SrsBasis::Lagrange { lagr_g1 } => &lagr_g1[offset..],
        SrsBasis::PowersOfTau { tau_powers_g1 } => &tau_powers_g1[offset..],
    };
    let commitment_hom: CommitmentHomomorphism<'_, E> = CommitmentHomomorphism {
        msm_basis,
        xi_1: ck.xi_1,
    };

    let input = Witness {
        hiding_randomness: r.clone(),
        values: Scalar::vec_from_inner_slice(&values[offset..]),
    };

    commitment_hom.apply(&input)
}

impl<'a, E: Pairing> CommitmentHomomorphism<'a, E> {
    pub fn open(
        ck: &CommitmentKey<E>,
        f_vals: Vec<E::ScalarField>, // needs to be evaluations of a polynomial f OR its coefficients, depending on `ck.msm_basis`
        rho: E::ScalarField,
        x: E::ScalarField,
        y: E::ScalarField,
        s: &CommitmentRandomness<E::ScalarField>,
    ) -> OpeningProof<E> {
        if ck.roots_of_unity_in_eval_dom.contains(&x) {
            panic!("x is not allowed to be a root of unity");
        }

        let q_vals = match &ck.msm_basis {
            SrsBasis::Lagrange { .. } => {
                // Lagrange basis expects f_vals to be evaluations, and we return q_vals with evaluations
                polynomials::quotient_evaluations_batch(
                    &f_vals,
                    &ck.roots_of_unity_in_eval_dom,
                    x,
                    y,
                )
            },
            SrsBasis::PowersOfTau { .. } => {
                // Powers-of-Tau expects f_vals to be coefficients, and we return q_vals with coefficients
                // For some reason arkworks only implemented `divide_with_q_and_r()` for `DenseOrSparsePolynomial`
                let f_dense = DensePolynomial { coeffs: f_vals };
                let f = DenseOrSparsePolynomial::DPolynomial(Cow::Owned(f_dense));
                let divisor_dense = DensePolynomial {
                    coeffs: vec![-x, E::ScalarField::ONE],
                };
                let divisor = DenseOrSparsePolynomial::DPolynomial(Cow::Owned(divisor_dense));

                let (q, _) = f.divide_with_q_and_r(&divisor).expect("Could not divide polynomial, but that shouldn't happen because the divisor is nonzero");
                q.coeffs
            },
        };

        let pi_1 = commit_with_randomness(ck, &q_vals, s);

        // For this small MSM, the direct approach seems to be faster than using `E::G1::msm()`
        let pi_2 = (ck.g1 * rho) - (ck.tau_1 - ck.g1 * x) * s.0;

        OpeningProof { pi_1, pi_2 }
    }

    #[allow(non_snake_case)]
    pub fn verify(
        vk: VerificationKey<E>,
        C: Commitment<E>,
        x: E::ScalarField,
        y: E::ScalarField,
        pi: OpeningProof<E>,
    ) -> anyhow::Result<()> {
        let VerificationKey {
            xi_2,
            tau_2,
            group_generators:
                GroupGenerators {
                    g1: one_1,
                    g2: one_2,
                },
        } = vk;
        let OpeningProof { pi_1, pi_2 } = pi;

        let check = E::multi_pairing(vec![C.0 - one_1 * y, -pi_1.0, -pi_2], vec![
            one_2,
            (tau_2 - one_2 * x).into_affine(),
            xi_2,
        ]);
        ensure!(
            PairingOutput::<E>::ZERO == check,
            "Hiding KZG verification failed"
        );

        Ok(())
    }
}

pub mod srs {
    use ark_ec::AffineRepr;
    use ark_serialize::CanonicalSerialize;
    use ark_serialize::SerializationError;
    use ark_serialize::Write;
    use ark_serialize::Valid;
    use ark_serialize::Compress;
    use ark_serialize::CanonicalDeserialize;
    use ark_serialize::Validate;
    use ark_serialize::Read;
    use ark_ec::CurveGroup;
    use aptos_crypto::utils;
    use ark_poly::EvaluationDomain;
    use ark_ff::Field;

    pub enum SrsType {
        Lagrange,
        PowersOfTau,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum SrsBasis<A: AffineRepr> {
        Lagrange { lagr_g1: Vec<A> },
        PowersOfTau { tau_powers_g1: Vec<A> },
    }

    impl<A: AffineRepr> CanonicalSerialize for SrsBasis<A> {
        fn serialize_with_mode<W: Write>(
            &self,
            mut writer: W,
            compress: Compress,
        ) -> Result<(), SerializationError> {
            match self {
                SrsBasis::Lagrange { lagr_g1 } => {
                    0u8.serialize_with_mode(&mut writer, compress)?; // variant tag
                    lagr_g1.serialize_with_mode(&mut writer, compress)?;
                },
                SrsBasis::PowersOfTau { tau_powers_g1 } => {
                    1u8.serialize_with_mode(&mut writer, compress)?; // variant tag
                    tau_powers_g1.serialize_with_mode(&mut writer, compress)?;
                },
            }
            Ok(())
        }

        fn serialized_size(&self, compress: Compress) -> usize {
            1 + match self {
                SrsBasis::Lagrange { lagr_g1 } => lagr_g1.serialized_size(compress),
                SrsBasis::PowersOfTau { tau_powers_g1 } => tau_powers_g1.serialized_size(compress),
            }
        }
    }

    impl<A: AffineRepr> Valid for SrsBasis<A> {
        fn check(&self) -> Result<(), SerializationError> {
            match self {
                SrsBasis::Lagrange { lagr_g1 } => {
                    for g in lagr_g1 {
                        g.check()?;
                    }
                },
                SrsBasis::PowersOfTau { tau_powers_g1 } => {
                    for g in tau_powers_g1 {
                        g.check()?;
                    }
                },
            }
            Ok(())
        }
    }

    impl<A: AffineRepr> CanonicalDeserialize for SrsBasis<A> {
        fn deserialize_with_mode<R: Read>(
            mut reader: R,
            compress: Compress,
            validate: Validate,
        ) -> Result<Self, SerializationError> {
            // Read the variant tag first
            let tag = u8::deserialize_with_mode(&mut reader, compress, validate)?;

            match tag {
                0 => {
                    // Lagrange variant
                    let lagr_g1 =
                        Vec::<A>::deserialize_with_mode(&mut reader, compress, validate)?;
                    Ok(SrsBasis::Lagrange { lagr_g1 })
                },
                1 => {
                    // Powers-of-Tau variant
                    let tau_powers_g1 =
                        Vec::<A>::deserialize_with_mode(&mut reader, compress, validate)?;
                    Ok(SrsBasis::PowersOfTau { tau_powers_g1 })
                },
                _ => Err(SerializationError::InvalidData),
            }
        }
    }

    pub fn lagrange_basis<C: CurveGroup>(
        n: usize,
        g1: C::Affine,
        eval_dom: ark_poly::Radix2EvaluationDomain<C::ScalarField>,
        tau: C::ScalarField,
    ) -> Vec<C::Affine> {
        let powers_of_tau = utils::powers(tau, n);
        let lagr_basis_scalars = eval_dom.ifft(&powers_of_tau);
        debug_assert!(lagr_basis_scalars.iter().sum::<C::ScalarField>() == C::ScalarField::ONE);

        let lagr_g1_proj: Vec<C> = lagr_basis_scalars.iter().map(|s| g1 * s).collect();
        C::normalize_batch(&lagr_g1_proj)
    }

    pub fn powers_of_tau<A: AffineRepr>(
        base: A,
        tau: A::ScalarField,
        m: usize,
    ) -> Vec<A> {
        let mut proj = Vec::with_capacity(m + 1);
        proj.push(base.into_group());
        for i in 0..m {
            proj.push(proj[i] * tau);
        }
        A::Group::normalize_batch(&proj)
    }
}

/// A fixed-base homomorphism used for computing commitments in the
/// *Hiding KZG (HKZG)* commitment scheme.
///
/// # Overview
///
/// This struct defines a homomorphism used to map scalars
/// (the polynomial evaluations and blinding factor) into an elliptic curve point,
/// producing a commitment in the HKZG scheme as (presumably) described in Zeromorph [^KT23e].
///
/// The homomorphism implements the following formula:
///
/// \\[
/// C = \rho \cdot \xi_1 + \sum_i f(\theta^i) \cdot \ell_i(\tau)_1
/// \\]
///
/// where:
/// - `ρ` is the blinding scalar,
/// - `ξ₁` is the fixed base obtained from a trapdoor `ξ`,
/// - `f(ωᵢ)` are polynomial evaluations at roots of unity ωᵢ,
/// - `ℓᵢ(τ)₁` are the Lagrange basis polynomials evaluated at trapdoor `τ`,
///
/// This homomorphism can be expressed as a *multi-scalar multiplication (MSM)*
/// over fixed bases, making it compatible with the `fixed_base_msms` framework.
///
///
/// # Fields
///
/// - `lagr_g1`: A slice of precomputed Lagrange basis elements \\(\ell_i(\tau) \cdot g_1\\),
///   used to commit to polynomial evaluations.
/// - `xi_1`: The base point corresponding to the blinding term \\(\xi_1 = ξ \cdot g_1\\).
///
///
/// # Implementation Notes
///
/// For consistency with `univariate_kzg.rs` and use in future sigma protocols, this implementation uses the
/// `fixed_base_msms::Trait` to express the homomorphism as a sequence of `(base, scalar)` pairs:
/// - The first pair encodes the hiding term `(ξ₁, ρ)`.
/// - The remaining pairs encode the polynomial evaluation commitments `(ℓᵢ(τ)₁, f(ωᵢ))`.
///
/// The MSM evaluation is then performed using `E::G1::msm()`.
///
/// TODO: Since this code is quite similar to that of ordinary KZG, it may be possible to reduce it a bit
#[derive(CanonicalSerialize, Debug, Clone, PartialEq, Eq)]
pub struct CommitmentHomomorphism<'a, E: Pairing> {
    pub msm_basis: &'a [E::G1Affine],
    pub xi_1: E::G1Affine,
}

#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct Witness<F: PrimeField> {
    pub hiding_randomness: CommitmentRandomness<F>,
    pub values: Vec<Scalar<F>>,
}

impl<E: Pairing> homomorphism::Trait for CommitmentHomomorphism<'_, E> {
    type Codomain = CodomainShape<E::G1>;
    type Domain = Witness<E::ScalarField>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        // CommitmentHomomorphism::<'_, E>::normalize_output(self.apply_msm(self.msm_terms(input)))
        self.apply_msm(self.msm_terms(input))
    }
}

impl<E: Pairing> fixed_base_msms::Trait for CommitmentHomomorphism<'_, E> {
    type Base = E::G1Affine;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = MsmInput<E::G1Affine, E::ScalarField>;
    type MsmOutput = E::G1;
    type Scalar = E::ScalarField;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        assert!(
            self.msm_basis.len() >= input.values.len(),
            "Not enough Lagrange basis elements for univariate hiding KZG: required {}, got {}",
            input.values.len(),
            self.msm_basis.len()
        );

        let mut scalars = Vec::with_capacity(input.values.len() + 1);
        scalars.push(input.hiding_randomness.0);
        scalars.extend(input.values.iter().map(|s| s.0.clone()));

        let mut bases = Vec::with_capacity(input.values.len() + 1);
        bases.push(self.xi_1);
        bases.extend(&self.msm_basis[..input.values.len()]);

        CodomainShape(MsmInput { bases, scalars })
    }

    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
        E::G1::msm(input.bases(), &input.scalars())
            .expect("MSM computation failed in univariate KZG")
    }

    fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
        E::G1::normalize_batch(&msm_output)
    }
}

impl<'a, E: Pairing> sigma_protocol::Trait<E::G1> for CommitmentHomomorphism<'a, E> {
    fn dst(&self) -> Vec<u8> {
        b"APTOS_HIDING_KZG_SIGMA_PROTOCOL_DST".to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::arkworks::random::{sample_field_element, sample_field_elements};
    use ark_ec::pairing::Pairing;
    use ark_poly::{univariate::DensePolynomial, Polynomial};
    use rand::thread_rng;

    // TODO: Should set up a PCS trait, then make these tests generic?
    fn assert_kzg_opening_correctness<E: Pairing>() {
        let mut rng = thread_rng();
        let group_data = GroupGenerators::default();

        type Fr<E> = <E as Pairing>::ScalarField;

        let m = 64;
        let xi = sample_field_element(&mut rng);
        let tau = sample_field_element(&mut rng);
        let (vk, ck) = setup::<E>(
            m,
            SrsType::Lagrange,
            group_data,
            Trapdoor { xi, tau },
        );

        let f_coeffs: Vec<Fr<E>> = sample_field_elements(m, &mut rng);
        let poly = DensePolynomial::<Fr<E>> { coeffs: f_coeffs };

        // Polynomial values at the roots of unity
        let f_evals: Vec<Fr<E>> = ck
            .roots_of_unity_in_eval_dom
            .iter()
            .map(|&gamma| poly.evaluate(&gamma))
            .collect();

        let rho = CommitmentRandomness::rand(&mut rng);
        let s = CommitmentRandomness::rand(&mut rng);
        let x = sample_field_element(&mut rng);
        let y =
            polynomials::barycentric_eval(&f_evals, &ck.roots_of_unity_in_eval_dom, x, ck.m_inv);

        // Commit to f
        let comm = super::commit_with_randomness(&ck, &f_evals, &rho);

        // Open at x, will fail when x is a root of unity but the odds of that should be negligible
        let proof = CommitmentHomomorphism::<E>::open(&ck, f_evals, rho.0, x, y, &s);

        // Verify proof
        let verification = CommitmentHomomorphism::<E>::verify(vk, comm, x, y, proof);

        assert!(
            verification.is_ok(),
            "Verification should succeed for correct proof"
        );
    }

    macro_rules! kzg_roundtrip_test {
        ($name:ident, $curve:ty) => {
            #[test]
            fn $name() {
                assert_kzg_opening_correctness::<$curve>();
            }
        };
    }

    kzg_roundtrip_test!(assert_kzg_opening_correctness_for_bn254, ark_bn254::Bn254);
    kzg_roundtrip_test!(
        assert_kzg_opening_correctness_for_bls12_381,
        ark_bls12_381::Bls12_381
    );
}
