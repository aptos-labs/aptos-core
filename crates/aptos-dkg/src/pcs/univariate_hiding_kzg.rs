// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::{polynomials, GroupGenerators},
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, fixed_base_msms::Trait as FixedBaseMsmsTrait, Trait},
    },
};
use anyhow::ensure;
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    AdditiveGroup, CurveGroup, VariableBaseMSM,
};
use ark_ff::Field;
use ark_poly::EvaluationDomain;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{CryptoRng, RngCore};
use sigma_protocol::homomorphism::TrivialShape as CodomainShape;

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct Commitment<E: Pairing>(pub E::G1);

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct CommitmentRandomness<E: Pairing>(pub E::ScalarField);

// TODO: Rename to OpeningProof?
#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct Proof<E: Pairing> {
    pi_1: Commitment<E>,
    pi_2: E::G1,
}

#[derive(CanonicalSerialize, Debug, Clone)]
pub struct VerificationKey<E: Pairing> {
    pub xi_2: E::G2Affine,
    pub tau_2: E::G2Affine,
    pub group_data: GroupGenerators<E>,
}

#[derive(CanonicalSerialize, Debug, Clone)]
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
    pub xi: E::ScalarField,
    pub tau: E::ScalarField,
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
    group_data: GroupGenerators<E>,
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
    } = group_data;
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
            group_data,
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

fn commit_with_randomness<E: Pairing>(
    ck: &CommitmentKey<E>,
    values: &[E::ScalarField],
    r: &CommitmentRandomness<E>,
) -> Commitment<E> {
    let commitment_hom: Homomorphism<'_, E> = Homomorphism {
        lagr_g1: &ck.lagr_g1,
        xi_1: ck.xi_1,
    };

    let input = (r.0, values.to_vec());

    Commitment(commitment_hom.apply(&input).0)
}

impl<'a, E: Pairing> Homomorphism<'a, E> {
    // TODO: should maybe make `y` part of the input, since it's often computed before invoking `open()`
    pub fn open(
        ck: &CommitmentKey<E>,
        f_evals: Vec<E::ScalarField>,
        rho: E::ScalarField,
        x: E::ScalarField,
        s: &CommitmentRandomness<E>,
    ) -> Proof<E> {
        if ck.roots_of_unity_in_eval_dom.contains(&x) {
            panic!("x is not allowed to be a root of unity");
        }

        let y =
            polynomials::barycentric_eval(&f_evals, &ck.roots_of_unity_in_eval_dom, x, ck.m_inv);

        let q_evals =
            polynomials::quotient_evaluations_batch(&f_evals, &ck.roots_of_unity_in_eval_dom, x, y);

        let pi_1 = commit_with_randomness(ck, &q_evals, s);

        let pi_2 = (ck.one_1 * rho) - (ck.tau_1 - ck.one_1 * x) * s.0;

        Proof { pi_1, pi_2 }
    }

    #[allow(non_snake_case)]
    pub fn verify(
        vk: VerificationKey<E>,
        C: Commitment<E>,
        x: E::ScalarField,
        y: E::ScalarField,
        pi: Proof<E>,
    ) -> anyhow::Result<()> {
        let VerificationKey {
            xi_2,
            tau_2,
            group_data:
                GroupGenerators {
                    g1: one_1,
                    g2: one_2,
                },
        } = vk;
        let Proof { pi_1, pi_2 } = pi;

        let check = E::multi_pairing(vec![C.0 - one_1 * y, -pi_1.0, -pi_2], vec![
            one_2,
            (tau_2 - one_2 * x).into_affine(),
            xi_2,
        ]);
        ensure!(PairingOutput::<E>::ZERO == check);

        Ok(())
    }
}

pub struct Homomorphism<'a, E: Pairing> {
    pub lagr_g1: &'a [E::G1Affine],
    pub xi_1: E::G1Affine,
}

impl<'a, E: Pairing> homomorphism::Trait for Homomorphism<'a, E> {
    type Codomain = CodomainShape<E::G1>;
    type Domain = (E::ScalarField, Vec<E::ScalarField>);

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

impl<'a, E: Pairing> fixed_base_msms::Trait for Homomorphism<'a, E> {
    type Base = E::G1Affine;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone;
    type MsmInput = homomorphism::fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
    type MsmOutput = E::G1;
    type Scalar = E::ScalarField;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        assert!(
            self.lagr_g1.len() >= input.1.len(),
            "Not enough Lagrange basis elements for univariate hiding KZG: required {}, got {}",
            input.1.len(),
            self.lagr_g1.len()
        );

        let mut scalars = Vec::with_capacity(input.1.len() + 1);
        scalars.push(input.0);
        scalars.extend_from_slice(&input.1);

        let mut bases = Vec::with_capacity(input.1.len() + 1);
        bases.push(self.xi_1);
        bases.extend(&self.lagr_g1[..input.1.len()]);

        CodomainShape(fixed_base_msms::MsmInput { bases, scalars })
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        E::G1::msm(bases, &scalars).expect("MSM failed in univariate KZG")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::{Bls12_381, Fr};
    use ark_poly::{univariate::DensePolynomial, Polynomial};
    use ark_std::{rand::thread_rng, UniformRand};

    // TODO: Should set up a PCS trait, then make this test generic? Also make it generic over E and then run it for BN254 and BLS12-381?
    #[allow(non_snake_case)]
    #[test]
    fn test_open_and_verify_roundtrip() {
        let mut rng = thread_rng();
        let group_data = GroupGenerators::sample(&mut rng);

        let m = 64;
        let xi = Fr::rand(&mut rng);
        let tau = Fr::rand(&mut rng);
        let (vk, ck) = setup::<Bls12_381, _>(m, group_data, Trapdoor { xi, tau }, &mut rng);

        let f_coeffs: Vec<Fr> = (0..m).map(|_| Fr::rand(&mut rng)).collect();
        let poly = DensePolynomial::<Fr> { coeffs: f_coeffs };

        // Polynomial values at the roots of unity
        let f_evals: Vec<Fr> = ck
            .roots_of_unity_in_eval_dom
            .iter()
            .map(|&gamma| poly.evaluate(&gamma))
            .collect();

        let rho = CommitmentRandomness::<Bls12_381>(Fr::rand(&mut rng));
        let s = CommitmentRandomness::<Bls12_381>(Fr::rand(&mut rng));
        let x = Fr::rand(&mut rng);
        let y =
            polynomials::barycentric_eval(&f_evals, &ck.roots_of_unity_in_eval_dom, x, ck.m_inv);

        // Commit to f
        let C = super::commit_with_randomness(&ck, &f_evals, &rho);

        // Open at x
        let proof = Homomorphism::<Bls12_381>::open(&ck, f_evals.clone(), rho.0, x, &s);

        // Verify proof
        let verification = Homomorphism::<Bls12_381>::verify(vk, C, x, y, proof);

        assert!(
            verification.is_ok(),
            "Verification should succeed for correct proof"
        );
    }
}
