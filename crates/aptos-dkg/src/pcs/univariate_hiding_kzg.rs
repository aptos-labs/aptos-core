// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::{polynomials, GroupGenerators},
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, fixed_base_msms::Trait as FixedBaseMsmsTrait, Trait},
    },
    Scalar,
};
use anyhow::ensure;
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    AdditiveGroup, CurveGroup, VariableBaseMSM,
};
use ark_ff::Field;
use ark_poly::EvaluationDomain;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{
    rand::{CryptoRng, Rng, RngCore},
    UniformRand,
};
use sigma_protocol::homomorphism::TrivialShape as CodomainShape;
use std::fmt::Debug;

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Commitment<E: Pairing>(pub E::G1);

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct CommitmentRandomness<E: Pairing>(pub E::ScalarField);

impl<E: Pairing> UniformRand for CommitmentRandomness<E> {
    fn rand<R: Rng + ?Sized>(rng: &mut R) -> Self {
        CommitmentRandomness(E::ScalarField::rand(rng))
    }
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, PartialEq, Eq, Clone)]
pub struct OpeningProof<E: Pairing> {
    pi_1: Commitment<E>,
    pi_2: E::G1,
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct VerificationKey<E: Pairing> {
    pub xi_2: E::G2Affine,
    pub tau_2: E::G2Affine,
    pub group_generators: GroupGenerators<E>,
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct CommitmentKey<E: Pairing> {
    pub xi_1: E::G1Affine,
    pub tau_1: E::G1Affine,
    pub lagr_g1: Vec<E::G1Affine>,
    pub eval_dom: ark_poly::Radix2EvaluationDomain<E::ScalarField>,
    pub roots_of_unity_in_eval_dom: Vec<E::ScalarField>,
    pub one_1: E::G1Affine,
    pub m_inv: E::ScalarField,
}

#[derive(CanonicalSerialize, Debug, Clone)]
pub struct Trapdoor<E: Pairing> {
    // Not sure this is the ideal location for tau...
    pub xi: E::ScalarField,
    pub tau: E::ScalarField,
}

impl<E: Pairing> Trapdoor<E> {
    pub fn rand<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            xi: E::ScalarField::rand(rng),
            tau: E::ScalarField::rand(rng),
        }
    }
}

pub fn lagrange_basis<E: Pairing>(
    n: usize,
    g1: E::G1Affine,
    eval_dom: ark_poly::Radix2EvaluationDomain<E::ScalarField>,
    tau: E::ScalarField,
) -> Vec<E::G1Affine> {
    let powers_of_tau = crate::utils::powers(tau, n);
    let lagr_basis_scalars = eval_dom.ifft(&powers_of_tau);
    debug_assert!(lagr_basis_scalars.iter().sum::<E::ScalarField>() == E::ScalarField::ONE);

    let lagr_g1_proj: Vec<E::G1> = lagr_basis_scalars.iter().map(|s| g1 * s).collect();
    E::G1::normalize_batch(&lagr_g1_proj)
}

pub fn setup<E: Pairing, R: RngCore + CryptoRng>(
    m: usize,
    group_generators: GroupGenerators<E>,
    trapdoor: Trapdoor<E>,
    _rng: &mut R,
) -> (VerificationKey<E>, CommitmentKey<E>) {
    assert!(
        m.is_power_of_two(),
        "Parameter m must be a power of 2, but got {}",
        m
    );

    let GroupGenerators {
        g1: one_1,
        g2: one_2,
    } = group_generators;
    let Trapdoor { xi, tau } = trapdoor;

    let xi_1 = (one_1 * xi).into_affine();
    let tau_1 = (one_1 * tau).into_affine();

    let xi_2 = (one_2 * xi).into_affine();
    let tau_2 = (one_2 * tau).into_affine();

    let eval_dom = ark_poly::Radix2EvaluationDomain::<E::ScalarField>::new(m)
        .expect("Could not construct evaluation domain");
    let lagr_g1 = lagrange_basis::<E>(m, one_1, eval_dom, tau);
    let roots_of_unity_in_eval_dom: Vec<E::ScalarField> = eval_dom.elements().collect();

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
            lagr_g1,
            eval_dom,
            roots_of_unity_in_eval_dom,
            one_1,
            m_inv,
        },
    )
}

pub fn commit_with_randomness<E: Pairing>(
    ck: &CommitmentKey<E>,
    values: &[E::ScalarField],
    r: &CommitmentRandomness<E>,
) -> Commitment<E> {
    let commitment_hom: CommitmentHomomorphism<'_, E> = CommitmentHomomorphism {
        lagr_g1: &ck.lagr_g1,
        xi_1: ck.xi_1,
    };

    let input = Witness {
        hiding_randomness: Scalar(r.0),
        values: Scalar::vec_from_inner_slice(values),
    };

    Commitment(commitment_hom.apply(&input).0)
}

impl<'a, E: Pairing> CommitmentHomomorphism<'a, E> {
    pub fn open(
        ck: &CommitmentKey<E>,
        f_evals: Vec<E::ScalarField>,
        rho: E::ScalarField,
        x: E::ScalarField,
        y: E::ScalarField,
        s: &CommitmentRandomness<E>,
    ) -> OpeningProof<E> {
        if ck.roots_of_unity_in_eval_dom.contains(&x) {
            panic!("x is not allowed to be a root of unity");
        }
        let q_evals =
            polynomials::quotient_evaluations_batch(&f_evals, &ck.roots_of_unity_in_eval_dom, x, y);

        let pi_1 = commit_with_randomness(ck, &q_evals, s);

        // For this small MSM, the direct approach seems to be faster than using `E::G1::msm()`
        let pi_2 = (ck.one_1 * rho) - (ck.tau_1 - ck.one_1 * x) * s.0;

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
    pub lagr_g1: &'a [E::G1Affine],
    pub xi_1: E::G1Affine,
}

#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct Witness<E: Pairing> {
    pub hiding_randomness: Scalar<E>,
    pub values: Vec<Scalar<E>>,
}

impl<E: Pairing> homomorphism::Trait for CommitmentHomomorphism<'_, E> {
    type Codomain = CodomainShape<E::G1>;
    type Domain = Witness<E>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

impl<E: Pairing> fixed_base_msms::Trait for CommitmentHomomorphism<'_, E> {
    type Base = E::G1Affine;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
    type MsmOutput = E::G1;
    type Scalar = E::ScalarField;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        assert!(
            self.lagr_g1.len() >= input.values.len(),
            "Not enough Lagrange basis elements for univariate hiding KZG: required {}, got {}",
            input.values.len(),
            self.lagr_g1.len()
        );

        let mut scalars = Vec::with_capacity(input.values.len() + 1);
        scalars.push(input.hiding_randomness.0);
        scalars.extend(input.values.iter().map(|s| s.0.clone()));

        let mut bases = Vec::with_capacity(input.values.len() + 1);
        bases.push(self.xi_1);
        bases.extend(&self.lagr_g1[..input.values.len()]);

        CodomainShape(fixed_base_msms::MsmInput { bases, scalars })
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        E::G1::msm(bases, &scalars).expect("MSM computation failed in univariate KZG")
    }
}

impl<'a, E: Pairing> sigma_protocol::Trait<E> for CommitmentHomomorphism<'a, E> {
    fn dst(&self) -> Vec<u8> {
        b"APTOS_HIDING_KZG_SIGMA_PROTOCOL_DST".to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ec::pairing::Pairing;
    use ark_poly::{univariate::DensePolynomial, Polynomial};
    use ark_std::{rand::thread_rng, UniformRand};

    // TODO: Should set up a PCS trait, then make these tests generic?
    fn assert_kzg_opening_correctness<E: Pairing>() {
        let mut rng = thread_rng();
        let group_data = GroupGenerators::sample(&mut rng);

        type Fr<E> = <E as Pairing>::ScalarField;

        let m = 64;
        let xi = Fr::<E>::rand(&mut rng);
        let tau = Fr::<E>::rand(&mut rng);
        let (vk, ck) = setup::<E, _>(m, group_data, Trapdoor { xi, tau }, &mut rng);

        let f_coeffs: Vec<Fr<E>> = (0..m).map(|_| Fr::<E>::rand(&mut rng)).collect();
        let poly = DensePolynomial::<Fr<E>> { coeffs: f_coeffs };

        // Polynomial values at the roots of unity
        let f_evals: Vec<Fr<E>> = ck
            .roots_of_unity_in_eval_dom
            .iter()
            .map(|&gamma| poly.evaluate(&gamma))
            .collect();

        let rho = CommitmentRandomness::<E>(Fr::<E>::rand(&mut rng));
        let s = CommitmentRandomness::<E>(Fr::<E>::rand(&mut rng));
        let x = Fr::<E>::rand(&mut rng);
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
