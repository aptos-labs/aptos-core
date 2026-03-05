// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Sigma protocol for the Shplonked ZK-PCS opening proof: proves knowledge of (rho, evals, u)
// such that com_y = commitment(rho, evals), V = taus_1[0]*sum(alphas_i*evals_i) + xi_1*u, and φ(y) = hom(y_rev, evals).
// Built from CurveGroupTupleHomomorphism (com_y, V) and a scalar homomorphism (SumHom or EvalHomLiftedOwned) via TupleHomomorphism.

// TODO: maybe this should go inside shplonked.rs as a submodule called sigma_protocol?

use crate::{
    pcs::{
        shplonked::{EvalPair, Srs},
        univariate_hiding_kzg,
    },
    sigma_protocol::{
        self,
        homomorphism::{
            self, fixed_base_msms, fixed_base_msms::Trait, tuple::CurveGroupTupleHomomorphism,
            Trait as HomTrait, TrivialShape as CodomainShape,
        },
        Trait as SigmaTrait, Witness,
    },
    Scalar,
};
use aptos_crypto::arkworks::{
    msm::MsmInput,
    random::{sample_field_element, sample_field_elements},
};
use ark_ec::{pairing::Pairing, CurveGroup, VariableBaseMSM};
use ark_ff::{PrimeField, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use homomorphism::tuple::TupleHomomorphism;
use rand_core::{CryptoRng, RngCore};
use std::{fmt::Debug, marker::PhantomData};

/// Witness for the Shplonked opening sigma protocol: (rho, evals, u) such that
/// C_y^hid = xi_1*rho + MSM(taus_1, hidden_evals), C_eval_hid = taus_1[0]*g_hid + xi_1*u, y_sum = sum(hidden_evals).
/// evals is per polynomial: { y_i^hid }_i. Full C_eval = C_eval_hid + g_rev·τ_0 is built by the verifier for the pairing check.
#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct ShplonkedSigmaWitness<F: PrimeField> {
    pub C_y_hid_randomness: F,
    /// Hidden evaluations per polynomial: { y_i^hid }_i.
    pub hidden_evals: Vec<Vec<F>>,
    pub C_evals_randomness: F,
}

impl<F: PrimeField> Witness<F> for ShplonkedSigmaWitness<F> {
    fn scaled_add(self, other: &Self, c: F) -> Self {
        let evals = self
            .hidden_evals
            .into_iter()
            .zip(other.hidden_evals.iter())
            .map(|(a, b)| {
                a.into_iter()
                    .zip(b.iter())
                    .map(|(x, y)| x + c * y)
                    .collect()
            })
            .collect();
        Self {
            C_y_hid_randomness: self.C_y_hid_randomness + c * other.C_y_hid_randomness,
            hidden_evals: evals,
            C_evals_randomness: self.C_evals_randomness + c * other.C_evals_randomness,
        }
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        Self {
            C_y_hid_randomness: sample_field_element(rng),
            hidden_evals: self
                .hidden_evals
                .iter()
                .map(|v| sample_field_elements(v.len(), rng))
                .collect(),
            C_evals_randomness: sample_field_element(rng),
        }
    }
}

fn project_to_kzg_witness<F: PrimeField>(
    w: &ShplonkedSigmaWitness<F>,
) -> univariate_hiding_kzg::Witness<F> {
    // To produce C_y^hid, we flatten the evals per polynomial into a single vector.
    let values: Vec<F> = w.hidden_evals.iter().flatten().cloned().collect();

    univariate_hiding_kzg::Witness {
        hiding_randomness: Scalar(w.C_y_hid_randomness),
        values: Scalar::vec_from_inner(values),
    }
}

/// Homomorphism for C_y^hid: (rho, evals, u) -> commitment(rho, evals). Ignores u.
pub type ComYHom<'a, E> = homomorphism::LiftHomomorphism<
    univariate_hiding_kzg::CommitmentHomomorphism<'a, E>,
    ShplonkedSigmaWitness<<E as Pairing>::ScalarField>,
>;

/// Builds the C_y^hid homomorphism (lifted commitment) using only the first `taus_1.len()` bases.
pub fn com_y_hom<'a, E: Pairing>(taus_1: &'a [E::G1Affine], xi_1: E::G1Affine) -> ComYHom<'a, E> {
    let inner = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
        msm_basis: taus_1,
        xi_1,
    };
    homomorphism::LiftHomomorphism {
        hom: inner,
        projection: project_to_kzg_witness::<E::ScalarField>,
    }
}

/// Homomorphism for C_eval_hid: g_hid·τ_0 + ρ_eval·ξ_1 where
/// g_hid = ∑_j weights[j] * (∑_i lagrange_at_x[j][i] * y_j^hid[i]) from the witness.
/// The verifier builds full C_eval = C_eval_hid + g_rev·τ_0 for the batch pairing check.
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct EvalPointCommitHom<E: Pairing> {
    pub tau_0: E::G1Affine,
    pub xi_1: E::G1Affine,
    /// One weight per polynomial (c^{j-1} Z_{S\\S_j}(x)).
    pub weights: Vec<E::ScalarField>,
    /// Lagrange basis at x per (j, i): lagrange_at_x[j][i] = L_{j,s_i}(x) for s_i in S_j^hid.
    /// We already computed the tilde_f_is in the main function, but we need to redo it here
    /// for the sigma proof.
    pub lagrange_at_x: Vec<Vec<E::ScalarField>>,
}

impl<E: Pairing> EvalPointCommitHom<E> {
    /// Build from SRS (uses only `taus_1[0]` and `xi_1`).
    #[allow(dead_code)]
    pub fn from_srs(
        srs: &Srs<E>,
        weights: Vec<E::ScalarField>,
        lagrange_at_x: Vec<Vec<E::ScalarField>>,
    ) -> Self {
        Self::new(srs.taus_1[0], srs.xi_1, weights, lagrange_at_x)
    }

    /// Build from the minimal bases needed: tau_0 and xi_1 (avoids passing the full SRS).
    pub fn new(
        tau_0: E::G1Affine,
        xi_1: E::G1Affine,
        weights: Vec<E::ScalarField>,
        lagrange_at_x: Vec<Vec<E::ScalarField>>,
    ) -> Self {
        Self {
            tau_0,
            xi_1,
            weights,
            lagrange_at_x,
        }
    }
}

impl<E: Pairing> HomTrait for EvalPointCommitHom<E> {
    type Codomain = CodomainShape<E::G1>;
    type CodomainNormalized = CodomainShape<E::G1Affine>;
    type Domain = ShplonkedSigmaWitness<E::ScalarField>;

    fn apply(&self, w: &Self::Domain) -> Self::Codomain {
        let input = self.msm_terms(w).0;
        let out = Self::msm_eval(input);
        CodomainShape(out)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        CodomainShape(value.0.into_affine())
    }
}

impl<E: Pairing> fixed_base_msms::Trait for EvalPointCommitHom<E> {
    type Base = E::G1Affine;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmOutput = E::G1;
    type Scalar = E::ScalarField;

    fn msm_terms(
        &self,
        witness: &Self::Domain,
    ) -> Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>> {
        // C_eval_hid = g_hid·τ_0 + ρ_eval·ξ_1
        debug_assert_eq!(
            self.weights.len(),
            witness.hidden_evals.len(),
            "weights and evals must have the same length (one per polynomial)"
        );
        debug_assert_eq!(
            self.lagrange_at_x.len(),
            witness.hidden_evals.len(),
            "lagrange_at_x and evals must have the same length"
        );
        let g_hid = witness
            .hidden_evals
            .iter()
            .zip(self.weights.iter())
            .zip(self.lagrange_at_x.iter())
            .map(|((y_j_hid, &w_j), l_j)| {
                debug_assert_eq!(
                    l_j.len(),
                    y_j_hid.len(),
                    "lagrange_at_x[j].len() == evals[j].len()"
                );
                let inner: E::ScalarField = y_j_hid
                    .iter()
                    .zip(l_j.iter())
                    .map(|(&y_ji, &l_ji)| l_ji * y_ji)
                    .fold(E::ScalarField::zero(), |a, b| a + b);
                w_j * inner
            })
            .fold(E::ScalarField::zero(), |a, b| a + b);
        let bases = vec![self.tau_0, self.xi_1];
        let scalars = vec![g_hid, witness.C_evals_randomness];
        CodomainShape(MsmInput::new(bases, scalars).expect("EvalPointCommitHom MSM"))
    }

    fn msm_eval(input: MsmInput<Self::Base, Self::Scalar>) -> Self::MsmOutput {
        E::G1::msm(input.bases(), input.scalars()).expect("EvalPointCommitHom msm_eval")
        // TODO: not sure we should be doing this because size-2 MSMs in arkworks might not be faster than elementwise multiplication
    }

    fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
        E::G1::normalize_batch(&msm_output)
    }
}

impl<E: Pairing> sigma_protocol::CurveGroupTrait for EvalPointCommitHom<E> {
    type Group = E::G1;

    fn dst(&self) -> Vec<u8> {
        b"ShplonkedSigma_VHom".to_vec()
    }
}

/// (com_y, V) as a curve-group tuple homomorphism.
pub type FirstTupleHom<'a, E> =
    CurveGroupTupleHomomorphism<<E as Pairing>::G1, ComYHom<'a, E>, EvalPointCommitHom<E>>;

/// Homomorphism for y_sum: (rho, evals, u) -> sum(evals). Used as third component with TupleHomomorphism.
#[derive(Clone, Debug, Default, PartialEq, Eq, CanonicalSerialize)]
pub struct SumHom<F: PrimeField>(PhantomData<F>);

impl<F: PrimeField> HomTrait for SumHom<F> {
    type Codomain = F;
    type CodomainNormalized = F;
    type Domain = ShplonkedSigmaWitness<F>;

    fn apply(&self, w: &Self::Domain) -> Self::Codomain {
        w.hidden_evals
            .iter()
            .flatten()
            .fold(F::zero(), |acc, x| acc + x)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        value
    }
}

impl<F: PrimeField> SigmaTrait for SumHom<F> {
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
        let sum_z = response
            .hidden_evals
            .iter()
            .flatten()
            .fold(F::zero(), |acc, x| acc + x);
        let expected = *prover_commitment + challenge * public_statement;
        anyhow::ensure!(sum_z == expected, "SumEvalsHom sigma check failed");
        Ok(())
    }
}

/// Lifts a homomorphism φ with domain [`EvalPair`] to the full sigma witness: projects
/// `ShplonkedSigmaWitness` to `(y_rev, hidden_evals)` and applies φ.
#[derive(CanonicalSerialize, Clone, Debug)]
pub struct EvalHomLifted<
    F: PrimeField,
    H: HomTrait<Domain = EvalPair<F>, Codomain = F, CodomainNormalized = F>,
> {
    /// Revealed evaluations per polynomial (public input to φ).
    pub y_rev: Vec<Vec<F>>,
    /// The homomorphism φ; apply(w) = hom.apply(&EvalPair { y_rev, y_hid: w.hidden_evals }).
    pub hom: H,
}

fn eval_pair_from_witness<F: PrimeField>(y_rev: &[Vec<F>], hidden_evals: &[Vec<F>]) -> EvalPair<F> {
    EvalPair {
        y_rev: y_rev.to_vec(),
        y_hid: hidden_evals.to_vec(),
    }
}

impl<F: PrimeField, H: HomTrait<Domain = EvalPair<F>, Codomain = F, CodomainNormalized = F>>
    HomTrait for EvalHomLifted<F, H>
{
    type Codomain = F;
    type CodomainNormalized = F;
    type Domain = ShplonkedSigmaWitness<F>;

    fn apply(&self, w: &Self::Domain) -> Self::Codomain {
        let pair = eval_pair_from_witness(&self.y_rev, &w.hidden_evals);
        self.hom.apply(&pair)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        value
    }
}

impl<F: PrimeField, H: HomTrait<Domain = EvalPair<F>, Codomain = F, CodomainNormalized = F>>
    SigmaTrait for EvalHomLifted<F, H>
{
    type Scalar = F;
    type VerifierBatchSize = usize;

    // Should not get used since this is a scalar homomorphism

    fn dst(&self) -> Vec<u8> {
        b"ShplonkedSigma_EvalHomLifted".to_vec()
    }

    fn verify_with_challenge<R: RngCore + CryptoRng>(
        &self,
        public_statement: &F,
        prover_commitment: &F,
        challenge: F,
        response: &ShplonkedSigmaWitness<F>,
        _verifier_batch_size: Option<Self::VerifierBatchSize>, // Should not get used since this is a scalar homomorphism
        _rng: &mut R,
    ) -> anyhow::Result<()> {
        let pair = eval_pair_from_witness(&self.y_rev, &response.hidden_evals);
        let phi_z = self.hom.apply(&pair);
        let expected = *prover_commitment + challenge * public_statement;
        anyhow::ensure!(
            phi_z == expected,
            "EvalHomLifted sigma check failed (φ(y_rev, z) = r + c·φ(y))"
        );
        Ok(())
    }
}

/// Full sigma homomorphism with scalar part = sum of hidden evals. Used for proof storage/deserialization.
pub type ShplonkedSigmaHom<'a, E> =
    TupleHomomorphism<FirstTupleHom<'a, E>, SumHom<<E as Pairing>::ScalarField>>;

/// Full sigma homomorphism with scalar part = arbitrary φ(y_rev, hidden_evals). Used at prove/verify.
pub type ShplonkedSigmaHomWithEval<'a, E, H> =
    TupleHomomorphism<FirstTupleHom<'a, E>, EvalHomLifted<<E as Pairing>::ScalarField, H>>;
