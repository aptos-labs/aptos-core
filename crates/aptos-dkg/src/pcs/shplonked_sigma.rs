// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Sigma protocol for the Shplonked ZK-PCS opening proof: proves knowledge of (rho, evals, u)
// such that com_y = commitment(rho, evals), V = taus_1[0]*sum(alpha_i*evals_i) + xi_1*u, and y_sum = sum(evals).
// Built from CurveGroupTupleHomomorphism (com_y, V) and SumHomomorphism (y_sum) via TupleHomomorphism.

use crate::{
    pcs::{shplonked::Srs, univariate_hiding_kzg},
    sigma_protocol::{
        self,
        homomorphism::{
            self, fixed_base_msms, tuple::CurveGroupTupleHomomorphism, Trait as HomTrait,
            TrivialShape as CodomainShape,
        },
        Trait as SigmaTrait, Witness,
    },
    Scalar,
};
use aptos_crypto::arkworks::random::sample_field_element;
use ark_ec::{pairing::Pairing, CurveGroup, VariableBaseMSM};
use ark_ff::{PrimeField, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::{CryptoRng, RngCore};
use std::fmt::Debug;

/// Witness for the Shplonked opening sigma protocol: (rho, evals, u) such that
/// com_y = xi_1*rho + MSM(taus_1, evals), V = taus_1[0]*sum(alpha_i*evals_i) + xi_1*u, y_sum = sum(evals).
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct ShplonkedSigmaWitness<F: PrimeField> {
    pub rho: F,
    pub evals: Vec<F>,
    pub u: F,
}

impl<F: PrimeField> Witness<F> for ShplonkedSigmaWitness<F> {
    fn scaled_add(self, other: &Self, c: F) -> Self {
        let evals = self
            .evals
            .into_iter()
            .zip(other.evals.iter())
            .map(|(a, b)| a + c * b)
            .collect();
        Self {
            rho: self.rho + c * other.rho,
            evals,
            u: self.u + c * other.u,
        }
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        Self {
            rho: sample_field_element(rng),
            evals: (0..self.evals.len())
                .map(|_| sample_field_element(rng))
                .collect(),
            u: sample_field_element(rng),
        }
    }
}

fn project_to_kzg_witness<F: PrimeField>(
    w: &ShplonkedSigmaWitness<F>,
) -> univariate_hiding_kzg::Witness<F> {
    univariate_hiding_kzg::Witness {
        hiding_randomness: Scalar(w.rho),
        values: Scalar::vec_from_inner(w.evals.clone()),
    }
}

/// Homomorphism for com_y: (rho, evals, u) -> commitment(rho, evals). Ignores u.
pub type ComYHom<'a, E> = homomorphism::LiftHomomorphism<
    univariate_hiding_kzg::CommitmentHomomorphism<'a, E>,
    ShplonkedSigmaWitness<<E as Pairing>::ScalarField>,
>;

/// Builds the com_y homomorphism (lifted commitment) for the given SRS.
pub fn com_y_hom<'a, E: Pairing>(srs: &'a Srs<E>) -> ComYHom<'a, E> {
    let inner = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
        msm_basis: &srs.taus_1,
        xi_1: srs.xi_1,
    };
    homomorphism::LiftHomomorphism {
        hom: inner,
        projection: project_to_kzg_witness::<E::ScalarField>,
    }
}

/// Homomorphism for V: (rho, evals, u) -> tau_0*sum(alpha_i*evals_i) + xi_1*u.
/// Parameterized by public weights alpha (derived from transcript). Owns bases for serialization.
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct VHom<E: Pairing> {
    pub tau_0: E::G1Affine,
    pub xi_1: E::G1Affine,
    pub alpha: Vec<E::ScalarField>,
}

impl<E: Pairing> VHom<E> {
    pub fn from_srs(srs: &Srs<E>, alpha: Vec<E::ScalarField>) -> Self {
        Self {
            tau_0: srs.taus_1[0],
            xi_1: srs.xi_1,
            alpha,
        }
    }
}

impl<E: Pairing> HomTrait for VHom<E> {
    type Codomain = CodomainShape<E::G1>;
    type CodomainNormalized = CodomainShape<E::G1Affine>;
    type Domain = ShplonkedSigmaWitness<E::ScalarField>;

    fn apply(&self, w: &Self::Domain) -> Self::Codomain {
        let sum_y = w
            .evals
            .iter()
            .zip(self.alpha.iter())
            .map(|(y, a)| *a * y)
            .fold(E::ScalarField::zero(), |a, b| a + b);
        let out = self.tau_0 * sum_y + self.xi_1 * w.u;
        CodomainShape(out)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        CodomainShape(value.0.into_affine())
    }
}

impl<E: Pairing> fixed_base_msms::Trait for VHom<E> {
    type Base = E::G1Affine;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmOutput = E::G1;
    type Scalar = E::ScalarField;

    fn msm_terms(
        &self,
        w: &Self::Domain,
    ) -> Self::CodomainShape<aptos_crypto::arkworks::msm::MsmInput<Self::Base, Self::Scalar>> {
        let sum_y = w
            .evals
            .iter()
            .zip(self.alpha.iter())
            .map(|(y, a)| *a * y)
            .fold(E::ScalarField::zero(), |a, b| a + b);
        let bases = vec![self.tau_0, self.xi_1];
        let scalars = vec![sum_y, w.u];
        CodomainShape(aptos_crypto::arkworks::msm::MsmInput::new(bases, scalars).expect("VHom MSM"))
    }

    fn msm_eval(
        input: aptos_crypto::arkworks::msm::MsmInput<Self::Base, Self::Scalar>,
    ) -> Self::MsmOutput {
        E::G1::msm(input.bases(), input.scalars()).expect("VHom msm_eval")
    }

    fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
        E::G1::normalize_batch(&msm_output)
    }
}

impl<E: Pairing> sigma_protocol::CurveGroupTrait for VHom<E> {
    type Group = E::G1;

    fn dst(&self) -> Vec<u8> {
        b"ShplonkedSigma_VHom".to_vec()
    }
}

/// (com_y, V) as a curve-group tuple homomorphism.
pub type ComYVHom<'a, E> = CurveGroupTupleHomomorphism<<E as Pairing>::G1, ComYHom<'a, E>, VHom<E>>;

/// Homomorphism for y_sum: (rho, evals, u) -> sum(evals). Used as third component with TupleHomomorphism.
#[derive(Clone, Debug, Default, PartialEq, Eq, CanonicalSerialize)]
pub struct SumEvalsHom<F: PrimeField>(std::marker::PhantomData<F>);

impl<F: PrimeField> HomTrait for SumEvalsHom<F> {
    type Codomain = F;
    type CodomainNormalized = F;
    type Domain = ShplonkedSigmaWitness<F>;

    fn apply(&self, w: &Self::Domain) -> Self::Codomain {
        w.evals.iter().fold(F::zero(), |acc, x| acc + x)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        value
    }
}

impl<F: PrimeField> SigmaTrait for SumEvalsHom<F> {
    type Scalar = F;
    type VerifierBatchSize = usize;

    fn dst(&self) -> Vec<u8> {
        b"ShplonkedSigma_SumEvalsHom".to_vec()
    }

    fn verify_with_challenge<R: RngCore + CryptoRng>(
        &self,
        public_statement: &F,
        prover_commitment: &F,
        challenge: F,
        response: &ShplonkedSigmaWitness<F>,
        _verifier_batch_size: Option<Self::VerifierBatchSize>,
        _rng: &mut R,
    ) -> anyhow::Result<()> {
        let sum_z = response.evals.iter().fold(F::zero(), |acc, x| acc + x);
        let expected = *prover_commitment + challenge * public_statement;
        anyhow::ensure!(sum_z == expected, "SumEvalsHom sigma check failed");
        Ok(())
    }
}

/// Full sigma homomorphism: ((com_y, V), y_sum).
pub type ShplonkedSigmaHom<'a, E> = homomorphism::tuple::TupleHomomorphism<
    ComYVHom<'a, E>,
    SumEvalsHom<<E as Pairing>::ScalarField>,
>;
