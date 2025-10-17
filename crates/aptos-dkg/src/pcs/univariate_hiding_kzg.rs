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
use ark_std::rand::{CryptoRng, RngCore};
use sigma_protocol::homomorphism::TrivialShape as CodomainShape;

#[derive(CanonicalSerialize, Debug, Clone)]
pub struct VerificationKey<E: Pairing> {
    pub(crate) xi_2: E::G2Affine,
    pub(crate) tau_2: E::G2Affine,
    pub(crate) group_data: GroupGenerators<E>,
}

#[derive(CanonicalSerialize, Debug, Clone)]
pub struct CommitmentKey<E: Pairing> {
    pub(crate) xi_1: E::G1Affine,
    pub(crate) tau_1: E::G1Affine,
    pub(crate) lagr_g1: Vec<E::G1Affine>,
    pub(crate) eval_dom: ark_poly::Radix2EvaluationDomain<E::ScalarField>, // not used in this file, but used elsewhere
    pub(crate) roots_of_unity_in_eval_dom: Vec<E::ScalarField>,
    pub(crate) one_1: E::G1Affine,
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
    let lagr_g1 = E::G1::normalize_batch(&lagr_g1_proj);
    lagr_g1
}

pub fn setup<E: Pairing, R: RngCore + CryptoRng>(
    m: usize,
    group_data: GroupGenerators<E>,
    xi: E::ScalarField,
    tau: E::ScalarField,
    _rng: &mut R,
) -> (VerificationKey<E>, CommitmentKey<E>) {
    let GroupGenerators {
        g1: one_1,
        g2: one_2,
    } = group_data;

    let xi_1 = (one_1 * xi).into_affine();
    let tau_1 = (one_1 * tau).into_affine();

    let xi_2 = (one_2 * xi).into_affine();
    let tau_2 = (one_2 * tau).into_affine();

    let eval_dom = ark_poly::Radix2EvaluationDomain::<E::ScalarField>::new(m)
        .expect("Could not construct evaluation domain");
    let lagr_g1 = lagrange_basis::<E>(m, one_1, eval_dom, tau);
    let roots_of_unity_in_eval_dom: Vec<E::ScalarField> = eval_dom.elements().collect();

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

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct Commitment<E: Pairing>(pub E::G1);

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct CommitmentRandomness<E: Pairing>(pub E::ScalarField);

#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct Proof<E: Pairing> {
    pi_1: Commitment<E>,
    pi_2: E::G1,
}

impl<'a, E: Pairing> Homomorphism<'a, E> {
    pub fn open(
        ck: &CommitmentKey<E>,
        f_evals: Vec<E::ScalarField>,
        rho: E::ScalarField,
        x: E::ScalarField,
        s: &CommitmentRandomness<E>,
    ) -> Proof<E> {
        if ck.roots_of_unity_in_eval_dom.contains(&x) {
            panic!("x is not allowed to be a root of unity");
        } // TODO: work with Result instead?

        let y = polynomials::barycentric_eval(&f_evals, &ck.roots_of_unity_in_eval_dom, x);

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
            "Not enough Lagrange basis elements for univariate KZG: required {}, got {}",
            input.1.len(),
            self.lagr_g1.len()
        );

        let mut scalars = Vec::with_capacity(input.1.len() + 1);
        scalars.push(input.0);
        scalars.extend_from_slice(&input.1);

        let mut bases = Vec::with_capacity(input.1.len() + 1);
        bases.push(self.xi_1);
        bases.extend(&self.lagr_g1[..input.1.len()]);

        CodomainShape(homomorphism::fixed_base_msms::MsmInput { bases, scalars })
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        E::G1::msm(bases, &scalars).expect("MSM failed in univariate KZG")
    }
}

// TODO: DO I NEED THE SIGMA STUFF? PROBABLY NOT

pub struct Sigma<'a, E: Pairing> {
    pub lagr_g1: &'a [E::G1Affine],
    pub xi_1: E::G1Affine,
}

#[derive(SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct Witness<E: Pairing> {
    pub randomness: Scalar<E>,
    pub values: Vec<Scalar<E>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::{Bls12_381, Fr, G1Projective, G2Projective};
    use ark_ec::{AffineRepr, PrimeGroup};
    use ark_poly::EvaluationDomain;
    use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
    use ark_std::{rand::thread_rng, test_rng};

    #[allow(non_snake_case)]
    #[test]
    fn test_open_and_verify_roundtrip() {
        let mut rng = thread_rng(); // not used
        let group_data = GroupGenerators::sample(&mut rng);

        let xi = Fr::from(11u64);
        let tau = Fr::from(13u64);
        let (vk, ck) = setup::<Bls12_381, _>(8, group_data, xi, tau, &mut rng);

        // Polynomial values at the roots of unity
        let f_evals: Vec<Fr> = ck
            .roots_of_unity_in_eval_dom
            .iter()
            .enumerate()
            .map(|(i, _)| Fr::from((i as u64) + 1))
            .collect();

        let rho = CommitmentRandomness::<Bls12_381>(Fr::from(5u64));
        let s = CommitmentRandomness::<Bls12_381>(Fr::from(2u64));
        let x = Fr::from(3u64);
        let y = polynomials::barycentric_eval(&f_evals, &ck.roots_of_unity_in_eval_dom, x);

        // Commit to f
        let C = super::commit_with_randomness(&ck, &f_evals, &rho);

        // Open at x
        let proof = Homomorphism::<Bls12_381>::open(&ck, f_evals.clone(), rho.0, x, &s);

        // Verify proof
        let result = Homomorphism::<Bls12_381>::verify(vk, C, x, y, proof);

        assert!(
            result.is_ok(),
            "Verification should succeed for correct proof"
        );
    }
}
