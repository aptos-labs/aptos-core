// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! SHPLONKeD opening proof: generalized batch opening with optional hiding and homomorphism φ.
//!
//! Implements PCS.BatchOpen and PCS.BatchVerify per the SHPLONKeD spec: batch opening of
//! univariate polynomials f₁,…,fₙ over evaluation sets S_i = S_i^rev ⊔ S_i^hid, with
//! revealed evaluations { y_i^rev }_i (one vector per polynomial), homomorphism image φ({ y_i }_i),
//! and commitment C_{y^hid} to hidden evaluations. Proof π = (π_1, π_2, C_{y^hid}, C_eval, π_PoK).
//! Notation: Z_S(X) = ∏_{s∈S}(X−s); y_i = (y_i^rev, y_i^hid); combined polynomial
//! f = ∑_i c^{i-1} Z_{S\S_i}(x) f_i − Z_S(x) q − g with g = ∑_i c^{i-1} Z_{S\S_i}(x) f̃_i(x);
//! opening at (x, 0); verifier computes C_f = ∑·C_i − Z_S·π_1 − C_eval + c^n·C_PoK.

// WARNING: THIS CODE HAS NOT BEEN PROPERLY VETTED, ONLY USE FOR BENCHMARKING PURPOSES!!!!!

use crate::{
    fiat_shamir::PolynomialCommitmentScheme as _,
    pcs::{
        shplonked_sigma::{self, ShplonkedSigmaWitness},
        traits::PolynomialCommitmentScheme,
        univariate_hiding_kzg::{self, Trapdoor},
        EvaluationSet,
    },
    sigma_protocol::{
        homomorphism::{
            fixed_base_msms::Trait as FixedBaseMsmsTrait, tuple::TupleCodomainShape, Trait as _,
            TrivialShape as CodomainShape,
        },
        CurveGroupTrait, FirstProofItem, Proof, Trait as SigmaTrait,
    },
    Scalar,
};
use aptos_crypto::{
    arkworks::{
        msm::{merge_scaled_msm_terms, MsmInput},
        random::sample_field_element,
        srs::{SrsBasis, SrsType},
        vanishing_poly, GroupGenerators,
    },
    utils::powers,
};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    AffineRepr, CurveGroup, VariableBaseMSM,
};
use ark_ff::{batch_inversion, AdditiveGroup, FftField, Field, Zero};
use ark_poly::{
    univariate::{DenseOrSparsePolynomial as DOSPoly, DensePolynomial},
    DenseUVPolynomial, EvaluationDomain, Polynomial, Radix2EvaluationDomain,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::{CryptoRng, RngCore};
#[cfg(any(
    feature = "pcs_verify_timing",
    feature = "range_proof_timing_multivariate"
))]
use std::time::{Duration, Instant};

/// Domain separation tag for the Shplonked opening sigma protocol (Fiat–Shamir context).
pub const SHPLONKED_SIGMA_DST: &[u8; 19] = b"Shplonked_Sigma_Dst";

/// Zero polynomial Z_S(X) = ∏_{s∈S}(X − s) for a set S.
#[allow(non_snake_case)]
pub fn zero_poly_S<F: FftField>(s: &[F]) -> DensePolynomial<F> {
    vanishing_poly::from_roots(s)
}

/// Set-theoretic union of all points from the given evaluation sets (no duplicates).
fn union_of_evaluation_sets<F: CanonicalSerialize + Eq + Clone>(
    sets: &[EvaluationSet<F>],
) -> Vec<F> {
    let mut out = Vec::new();
    for set in sets.iter() {
        for p in set.all_points() {
            let p = p.clone();
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// Returns Z_{S \ S_i}(x) for each i: i.e. Z_S(x) / Z_{S_i}(x).
/// Uses direct evaluation: Z_S(x) once, then per i compute Z_{S_i}(x) = ∏_{s∈S_i}(x−s) and divide.
/// Cost O(|S|) + O(∑_i |S_i|) instead of polynomial division per i.
#[allow(non_snake_case)]
fn evaluate_z_S_minus_S_is<F: Field>(z_S_at_x: F, s_per_poly: &[impl AsRef<[F]>], x: F) -> Vec<F> {
    let mut z_S_i_vals: Vec<F> = s_per_poly
        .iter()
        .map(|s_i| {
            let s_i = s_i.as_ref();
            if s_i.is_empty() {
                F::ONE // placeholder so batch_inversion does not invert zero
            } else {
                s_i.iter().map(|&s| x - s).product()
            }
        })
        .collect();
    batch_inversion(&mut z_S_i_vals);
    z_S_i_vals.into_iter().map(|inv| z_S_at_x * inv).collect()
}

/// Lagrange basis: L_{i,s}(x) for a single s in set S_i (1 at s, 0 at other points of S_i).
#[allow(non_snake_case)]
pub fn lagrange_basis_at<F: Field>(s_i: &[F], s: F, x: F) -> F {
    let mut num = F::one();
    let mut den = F::one();
    for &t in s_i {
        if t == s {
            continue;
        }
        num *= x - t;
        den *= s - t;
    }
    num * den
        .inverse()
        .expect("denominator nonzero for distinct points")
}

/// Builds Lagrange basis polynomials given pre-inverted denominators (L_s = (Z_{S_i}(X)/(X-s)) * inv_s).
#[allow(non_snake_case)]
fn lagrange_basis_polys_from_inverted_denoms<F: FftField>(
    s_i: &[F],
    inv_denoms: &[F],
) -> Vec<DensePolynomial<F>> {
    debug_assert_eq!(s_i.len(), inv_denoms.len());
    if s_i.is_empty() {
        return Vec::new();
    }
    let z_S_i = vanishing_poly::from_roots(s_i);
    let z_S_i_dos = DOSPoly::from(z_S_i.clone());
    s_i.iter()
        .enumerate()
        .map(|(idx, &s)| {
            let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![-s, F::one()]));
            let (l_s_poly, r) = z_S_i_dos.clone().divide_with_q_and_r(&divisor).unwrap();
            debug_assert!(r.is_zero());
            let mut l_s: DensePolynomial<F> = l_s_poly.into();
            l_s = &l_s * inv_denoms[idx];
            l_s
        })
        .collect()
}

/// Lagrange basis polynomials for multiple point sets with a single batch inversion across all denominators.
/// Returns one `Vec<DensePolynomial<F>>` per set in `unique_sets`.
#[allow(non_snake_case)]
fn lagrange_basis_polys_batched<F: FftField>(unique_sets: &[&[F]]) -> Vec<Vec<DensePolynomial<F>>> {
    if unique_sets.is_empty() {
        return Vec::new();
    }
    let mut all_denoms: Vec<F> = Vec::new();
    for s_i in unique_sets.iter() {
        for &s in s_i.iter() {
            let denom = s_i
                .iter()
                .filter(|&&t| t != s) // Can we rule this out?
                .fold(F::one(), |a, &t| a * (s - t));
            all_denoms.push(denom);
        }
    }
    batch_inversion(&mut all_denoms);
    let mut offset = 0;
    unique_sets
        .iter()
        .map(|s_i| {
            let len = s_i.len();
            let inv_denoms = &all_denoms[offset..offset + len];
            offset += len;
            lagrange_basis_polys_from_inverted_denoms(s_i, inv_denoms)
        })
        .collect()
}

/// Homomorphism φ on the evaluations { y_i }_i, where y_i = (y_i^rev, y_i^hid). The prover reveals
/// { y_i^rev }_i and φ({ y_i }_i); the sigma protocol proves knowledge of { y_i^hid }_i consistent
/// with C_{y^hid}, C_eval, and φ({ y_i }_i).
pub trait EvalHomomorphism<F: Field>: Send + Sync {
    /// φ({ y_i }_i) with y_rev = { y_i^rev }_i and y_hid = { y_i^hid }_i (one vector per polynomial each).
    fn apply(&self, y_rev: &[Vec<F>], y_hid: &[Vec<F>]) -> F;
}

/// Default: φ(y) = ∑_j y_j (sum of all evaluations).
#[derive(Clone, Debug, Default)]
pub struct SumEvalHom;

impl<F: Field> EvalHomomorphism<F> for SumEvalHom {
    fn apply(&self, y_rev: &[Vec<F>], y_hid: &[Vec<F>]) -> F {
        y_rev
            .iter()
            .flatten()
            .chain(y_hid.iter().flatten())
            .fold(F::zero(), |a, &b| a + b)
    }
}

/// Builds canonical indices and Lagrange basis cache for the given point sets (one per polynomial).
/// Returns (canonical, lagrange_cache) where lagrange_cache[i] is Some(bases) when canonical[i] == i.
#[allow(non_snake_case)]
fn build_lagrange_cache<F: FftField>(
    s_per_poly: &[Vec<F>],
) -> (Vec<usize>, Vec<Option<Vec<DensePolynomial<F>>>>) {
    let n = s_per_poly.len();
    let canonical: Vec<usize> = (0..n)
        .map(|i| (0..=i).find(|&j| s_per_poly[j] == s_per_poly[i]).unwrap())
        .collect();
    let unique_indices: Vec<usize> = (0..n).filter(|&i| canonical[i] == i).collect();
    let unique_sets: Vec<&[F]> = unique_indices
        .iter()
        .map(|&i| s_per_poly[i].as_slice())
        .collect();
    let lagrange_bases_batched = lagrange_basis_polys_batched(&unique_sets);
    let mut lagrange_cache: Vec<Option<Vec<DensePolynomial<F>>>> = vec![None; n];
    for (bases, &idx) in lagrange_bases_batched
        .into_iter()
        .zip(unique_indices.iter())
    {
        lagrange_cache[idx] = Some(bases);
    }
    (canonical, lagrange_cache)
}

// ---------------------------------------------------------------------------
// SRS and legacy API (single point per polynomial, φ = sum)
// ---------------------------------------------------------------------------

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Srs<E: Pairing> {
    pub(crate) taus_1: Vec<E::G1Affine>,
    pub(crate) xi_1: E::G1Affine,
    pub(crate) g_2: E::G2Affine,
    pub(crate) tau_2: E::G2Affine,
    pub(crate) xi_2: E::G2Affine,
}

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Clone, CanonicalDeserialize, Debug, PartialEq, Eq)]
pub(crate) struct ShplonkedSigmaProof<E: Pairing> {
    r_com_y: E::G1Affine,
    r_V: E::G1Affine,
    r_y: E::ScalarField,
    /// Sigma protocol response: hidden evals per polynomial { z_i }_i (same shape as { y_i^hid }_i).
    z_yi: Vec<Vec<E::ScalarField>>,
    z_u: E::ScalarField,
    z_rho: E::ScalarField,
}

/// Statement for the sigma protocol: commitment to hidden evaluations (C_{y^hid}), C_eval, and φ(y).
#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Clone, CanonicalDeserialize, Debug, PartialEq, Eq)]
pub struct ShplonkedSigmaProofStatement<E: Pairing> {
    /// C_{y^hid}: commitment to hidden evaluations.
    pub com_y_hid: E::G1Affine,
    /// C_eval: [∑_i c^{i-1} Z_{S\S_i}(x) f̃_i(x)]_1.
    pub C_eval: E::G1Affine,
    /// φ(y) (e.g. sum of all evaluations).
    pub phi_y: E::ScalarField,
    /// Alias for φ(y) for legacy API (e.g. Dekart batch check: y_sum == y_batched_at_z + y_g).
    pub y_sum: E::ScalarField,
}

/// Generalized batch opening proof per spec: π = (π_1, π_2, π_PoK).
/// π_1 = commitment to q; π_2 = opening at x (quotient commitment and hiding compensation).
/// C_{y^hid}, C_eval, φ(y) live in sigma_proof_statement.
#[allow(non_snake_case)]
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct ShplonkedBatchProof<E: Pairing> {
    /// π_1: commitment to quotient polynomial q (W in legacy naming).
    pub pi_1: E::G1Affine,
    /// π_2: quotient commitment from PCS.Open (opening at x).
    pub pi_2: (E::G1Affine, E::G1Affine),
    /// π_PoK: sigma protocol proof of knowledge of y^hid.
    pub sigma_proof: ShplonkedSigmaProof<E>,
    /// Statement for sigma verify and legacy API (com_y_hid, C_eval, phi_y / y_sum).
    pub sigma_proof_statement: ShplonkedSigmaProofStatement<E>,
}

/// Batch opening: revealed evaluations plus the batch proof.
/// Output is { y_i^rev }_i (one vector of revealed evals per polynomial).
#[allow(non_snake_case)]
#[derive(Clone, Debug)]
pub struct ShplonkedBatchOpening<E: Pairing> {
    /// Revealed evaluations { y_i^rev }_i: for each polynomial i, the evaluations at S_i^rev.
    pub evals: Vec<Vec<E::ScalarField>>,
    /// Batch opening proof π.
    pub proof: ShplonkedBatchProof<E>,
}

impl<E: Pairing> ShplonkedBatchOpening<E> {
    /// Returns the revealed evaluations per polynomial: { y_i^rev }_i.
    pub fn get_evals(&self) -> &[Vec<E::ScalarField>] {
        &self.evals
    }

    /// Returns φ(y) (e.g. sum of all evaluations).
    pub fn get_phi_eval(&self) -> E::ScalarField {
        self.proof.sigma_proof_statement.phi_y
    }
}

/// Builds a commitment key from the Shplonked SRS for use with PCS.Open (PowersOfTau basis).
#[allow(non_snake_case)]
fn commitment_key_from_srs<E: Pairing>(srs: &Srs<E>) -> univariate_hiding_kzg::CommitmentKey<E>
where
    E::ScalarField: FftField,
{
    let m = srs.taus_1.len();
    let eval_dom = Radix2EvaluationDomain::<E::ScalarField>::new(m)
        .expect("SRS size m must be a power of two");
    let roots_of_unity_in_eval_dom = eval_dom.elements().collect();
    let m_inv = E::ScalarField::from(m as u64).inverse().expect("m nonzero");
    univariate_hiding_kzg::CommitmentKey {
        xi_1: srs.xi_1,
        tau_1: srs.taus_1[1],
        msm_basis: SrsBasis::PowersOfTau {
            tau_powers: srs.taus_1.clone(),
        },
        eval_dom,
        roots_of_unity_in_eval_dom,
        g1: srs.taus_1[0],
        m_inv,
    }
}

/// Appends the batch-open statement to the transcript for Fiat–Shamir (Step 1b).
fn append_batch_statement_to_transcript<E: Pairing>(
    trs: &mut merlin::Transcript,
    sets: &[EvaluationSet<E::ScalarField>],
    y_rev: &[Vec<E::ScalarField>],
    phi_y: E::ScalarField,
    c_y_hid: &E::G1Affine,
) {
    for set in sets {
        trs.append_evaluation_set(set);
    }
    for y_i_rev in y_rev {
        trs.append_evaluation_points(y_i_rev);
    }
    trs.append_homomorphism_image(&phi_y);
    trs.append_point(c_y_hid);
}

/// Computes Z_S(x) and weights c_powers[i] * Z_{S\S_i}(x).
#[allow(non_snake_case)]
fn compute_weights<E: Pairing>(
    z_S: &DensePolynomial<E::ScalarField>,
    s_per_poly: &[Vec<E::ScalarField>],
    x: E::ScalarField,
    c_powers: &[E::ScalarField],
) -> (E::ScalarField, Vec<E::ScalarField>) {
    let z_S_val = z_S.evaluate(&x);
    let z_S_minus_S_i_vals = evaluate_z_S_minus_S_is(z_S_val, s_per_poly, x);

    debug_assert_eq!(c_powers.len(), z_S_minus_S_i_vals.len());
    let weights: Vec<E::ScalarField> = c_powers
        .iter()
        .zip(z_S_minus_S_i_vals.iter())
        .map(|(&c_i, &z_val)| c_i * z_val)
        .collect();
    (z_S_val, weights)
}

/// Generalized batch open per spec: PCS.BatchOpen(prk, {S_i}, φ; {f_i}, {ρ_i}) → (y^rev, φ(y), π).
/// Supports arbitrary evaluation sets S_i = S_i^rev ⊔ S_i^hid and homomorphism φ.
/// Here S = ⋃_i S_i (spec: Z_S is the zero polynomial of S); typically S_i are disjoint.
#[allow(non_snake_case)]
pub fn batch_open_generalized<
    E: Pairing,
    R: RngCore + CryptoRng,
    H: EvalHomomorphism<E::ScalarField>,
>(
    srs: &Srs<E>,
    sets: &[EvaluationSet<E::ScalarField>],
    polys: &[DensePolynomial<E::ScalarField>],
    rhos: &[E::ScalarField], // commitment randomness for each polynomial
    hom: &H,
    trs: &mut merlin::Transcript,
    rng: &mut R,
) -> ShplonkedBatchOpening<E> {
    let n = polys.len();
    assert_eq!(sets.len(), n);
    assert_eq!(rhos.len(), n);

    // Step 1a: Compute { y_i }_i = (y_i^rev, y_i^hid) per polynomial, then φ({ y_i }_i) and C_{y^hid}.
    let mut y_rev_per_poly: Vec<Vec<E::ScalarField>> = Vec::with_capacity(n);
    let mut y_hid_per_poly: Vec<Vec<E::ScalarField>> = Vec::with_capacity(n);
    let mut evals_per_poly: Vec<Vec<E::ScalarField>> = Vec::with_capacity(n); // a bit inefficient constructing all three, but shoulnd't be an issue atm
    for (set, poly) in sets.iter().zip(polys.iter()) {
        let s_i: Vec<_> = set.all_points().cloned().collect();
        let evals_i: Vec<_> = s_i.iter().map(|s| poly.evaluate(s)).collect();
        evals_per_poly.push(evals_i.clone());
        let n_rev = set.rev.len();
        y_rev_per_poly.push(evals_i.iter().take(n_rev).cloned().collect());
        y_hid_per_poly.push(evals_i.iter().skip(n_rev).cloned().collect());
    }
    let y_hid_flat: Vec<E::ScalarField> = y_hid_per_poly.iter().flatten().cloned().collect();
    let phi_y = hom.apply(&y_rev_per_poly, &y_hid_per_poly);

    let c_y_hid_randomness = sample_field_element(rng);
    let com_y_hom = shplonked_sigma::com_y_hom::<E>(&srs.taus_1[..y_hid_flat.len()], srs.xi_1);
    let com_y_hid = com_y_hom
        .apply(&shplonked_sigma::ShplonkedSigmaWitness {
            C_y_hid_randomness: c_y_hid_randomness,
            evals: y_hid_per_poly.clone(),
            C_evals_randomness: E::ScalarField::zero(),
        })
        .0
        .into_affine();

    // Step 1b: Add { S_i }_i, { y_i^rev }_i, φ({ y_i }_i), C_{y^hid} to transcript.
    append_batch_statement_to_transcript::<E>(trs, sets, &y_rev_per_poly, phi_y, &com_y_hid);

    // Step 1c: Derive a challenge c from the Fiat-Shamir transcript.
    let c: E::ScalarField = trs.challenge_scalar();
    let c_powers = powers(c, n);

    // Step 2a: Compute q(X) = ∑_{i=1}^n c^{i-1} (f_i(X) − f̃_i(X)) / Z_{S_i}(X).
    // All evaluation sets S_i must be nonempty, for convenience.
    for set in sets.iter() {
        assert!(!set.is_empty(), "all evaluation sets S_i must be nonempty");
    }
    let s_union = union_of_evaluation_sets(sets);
    let z_S = zero_poly_S(&s_union);
    // One Vec per set (rev ∥ hid) so we have &[F] for from_roots, tilde_f_i_poly, etc.
    // Same point data as in `sets`, just materialized as contiguous slices for convenience.
    let S_is: Vec<Vec<E::ScalarField>> = sets
        .iter()
        .map(|set| set.all_points().cloned().collect())
        .collect();

    // Canonical index per polynomial and Lagrange basis cache (one batch inversion for all denominators).
    let (canonical, lagrange_cache) = build_lagrange_cache(&S_is);

    let tilde_f_is: Vec<DensePolynomial<E::ScalarField>> = (0..n)
        .map(|i| {
            let bases = lagrange_cache[canonical[i]].as_ref().unwrap();
            bases
                .iter()
                .zip(evals_per_poly[i].iter())
                .map(|(l_s, &y)| l_s * y)
                .fold(DensePolynomial::zero(), |a, b| &a + &b)
        })
        .collect();

    let f_is: Vec<DensePolynomial<E::ScalarField>> = polys
        .iter()
        .map(|p| DensePolynomial::from_coefficients_vec(p.coeffs().to_vec()))
        .collect();

    let mut z_S_is: Vec<DensePolynomial<E::ScalarField>> = Vec::with_capacity(n);
    for i in 0..n {
        let z = if canonical[i] == i {
            vanishing_poly::from_roots(&S_is[i])
        } else {
            z_S_is[canonical[i]].clone() // TODO: might not be necessary, just use canonical[..] again
        };
        z_S_is.push(z);
    }

    let mut q_poly = DensePolynomial::zero();
    for i in 0..n {
        let diff = &f_is[i] - &tilde_f_is[i];
        let (q_i_dos, remainder) = DOSPoly::from(diff.clone())
            .divide_with_q_and_r(&DOSPoly::from(&z_S_is[canonical[i]]))
            .expect("Z_{S_i} divides (f_i − f̃_i)");
        debug_assert!(remainder.is_zero());
        let q_i: DensePolynomial<E::ScalarField> = q_i_dos.into();
        q_poly += &(q_i * c_powers[i]);
    }

    // Step 2b: Sample commitment randomness rho_q.
    let rho_q = sample_field_element(rng);

    // Step 2c: Compute π_1 (commitment to q uses full SRS; q can have larger degree than h).
    let hom_commit_q = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
        msm_basis: &srs.taus_1,
        xi_1: srs.xi_1,
    };
    let pi_1 = hom_commit_q
        .apply(&univariate_hiding_kzg::Witness {
            hiding_randomness: Scalar(rho_q),
            values: Scalar::vec_from_inner(q_poly.coeffs().to_vec()),
        })
        .0
        .into_affine();

    // Step 2d: Add π_1 to the Fiat-Shamir transcript.
    trs.append_point(&pi_1);

    // Step 3: Derive a challenge x from the Fiat-Shamir transcript.
    let x: E::ScalarField = trs.challenge_scalar();

    // Step 4a: Sample commitment randomness ρ_eval.
    let rho_eval = sample_field_element(rng);

    // Step 4b: Compute all weights c^{i-1} Z_{S\S_i}(x), then g
    let (z_S_val, weights) = compute_weights::<E>(&z_S, &S_is, x, &c_powers);

    let g: E::ScalarField = (0..n)
        .map(|i| weights[i] * tilde_f_is[i].evaluate(&x))
        .sum();

    // Step 4c: C_eval = eval_point_commit_hom(y^hid; ρ_eval) = (∑_j weights[j] * (∑_i L_{j,i}(x) y_j^hid[i]))*τ_0 + ρ_eval*ξ_1.
    // Use already-computed Lagrange basis polynomials and evaluate at x (Horner).
    let lagrange_at_x: Vec<Vec<E::ScalarField>> = (0..n)
        .map(|j| {
            let bases = lagrange_cache[canonical[j]].as_ref().unwrap();
            let n_rev = sets[j].rev.len();
            (0..sets[j].hid.len())
                .map(|i| bases[n_rev + i].evaluate(&x))
                .collect()
        })
        .collect();
    let eval_point_commit_hom = shplonked_sigma::EvalPointCommitHom::new(
        srs.taus_1[0],
        srs.xi_1,
        weights.clone(),
        lagrange_at_x,
    );
    let witness_for_C_eval = shplonked_sigma::ShplonkedSigmaWitness {
        C_y_hid_randomness: E::ScalarField::zero(), // We're using the full sigma protocol witness here which is a bit awkward; but fine if we kill off this component
        evals: y_hid_per_poly.clone(),
        C_evals_randomness: rho_eval,
    };
    let C_eval_proj: E::G1 = eval_point_commit_hom.apply(&witness_for_C_eval).0;
    let C_eval = C_eval_proj.into_affine();

    // Step 5a: f = ∑_i c^{i-1} Z_{S\S_i}(x) f_i − Z_S(x) q − g.
    let mut f_poly = DensePolynomial::zero();
    for i in 0..n {
        f_poly += &(&f_is[i] * weights[i]);
    }
    f_poly -= &(&q_poly * z_S_val);
    f_poly -= &DensePolynomial::from_coefficients_vec(vec![g]);

    // Step 5b: ρ = ∑_i c^{i-1} Z_{S\S_i}(x) ρ_i − Z_S(x) ρ_q − ρ_eval.
    let mut rho = E::ScalarField::zero();
    for i in 0..n {
        rho += weights[i] * rhos[i];
    }
    rho -= z_S_val * rho_q;
    rho -= rho_eval;

    // Step 5c: π₂ ← PCS.Open(prk, f, x; ρ). Opening is at (x, 0) since f(x) = 0.
    let ck = commitment_key_from_srs::<E>(srs);
    let s = sample_field_element(rng);
    let opening = univariate_hiding_kzg::CommitmentHomomorphism::<E>::open(
        &ck,
        f_poly.coeffs().to_vec(),
        rho,
        x,
        E::ScalarField::zero(),
        &Scalar(s),
        0,
    );

    // Step 5d: compute the sigma proof
    let witness = ShplonkedSigmaWitness {
        C_y_hid_randomness: c_y_hid_randomness,
        evals: y_hid_per_poly.clone(),
        C_evals_randomness: rho_eval,
    };
    let com_y_v_hom = shplonked_sigma::FirstTupleHom::<E> {
        hom1: com_y_hom,
        hom2: eval_point_commit_hom,
        _group: std::marker::PhantomData,
    };
    let sum_hom = shplonked_sigma::SumHom::<E::ScalarField>::default();
    let full_hom = shplonked_sigma::ShplonkedSigmaHom::<E> {
        hom1: com_y_v_hom,
        hom2: sum_hom,
    };
    let statement_proj = TupleCodomainShape(
        TupleCodomainShape(CodomainShape(com_y_hid.into()), CodomainShape(C_eval_proj)),
        phi_y,
    );
    let (sigma_protocol_proof, _) =
        full_hom.prove(&witness, statement_proj, SHPLONKED_SIGMA_DST, rng);
    let (r_com_y, r_V, r_y) = match &sigma_protocol_proof.first_proof_item {
        FirstProofItem::Commitment(c) => (c.0 .0 .0, c.0 .1 .0, c.1),
        FirstProofItem::Challenge(_) => panic!("expected commitment"),
    };
    let sigma_proof = ShplonkedSigmaProof {
        // TODO: should probably get rid of this stuff
        r_com_y,
        r_V,
        r_y,
        z_yi: sigma_protocol_proof.z.evals,
        z_u: sigma_protocol_proof.z.C_evals_randomness,
        z_rho: sigma_protocol_proof.z.C_y_hid_randomness,
    };

    // π_2 from PCS.Open: pi_2 = quotient commitment, pi_2_extra = hiding compensation.
    let pi_2 = opening.pi_1.0.into_affine();
    let pi_2_extra = opening.pi_2.into_affine();

    let sigma_proof_statement = ShplonkedSigmaProofStatement {
        com_y_hid,
        C_eval,
        phi_y,
        y_sum: phi_y,
    };

    let proof = ShplonkedBatchProof {
        pi_1,
        pi_2: (pi_2, pi_2_extra),
        sigma_proof,
        sigma_proof_statement,
    };
    ShplonkedBatchOpening {
        evals: y_rev_per_poly,
        proof,
    }
}

/// Generalized batch verify per spec: PCS.BatchVerify(vk, {S_i}, φ, {C_i}; { y_i^rev }_i, φ(y), π) → {0,1}.
/// Commitments C_i may be given as MSM representations (they are expanded into the equation).
#[allow(non_snake_case)]
pub fn batch_verify_generalized<
    E: Pairing,
    R: RngCore + CryptoRng,
    H: EvalHomomorphism<E::ScalarField>,
>(
    srs: &Srs<E>,
    sets: &[EvaluationSet<E::ScalarField>],
    hom: &H,
    commitment_msms: &[MsmInput<E::G1Affine, E::ScalarField>],
    y_rev: &[Vec<E::ScalarField>],
    phi_y: E::ScalarField,
    proof: &ShplonkedBatchProof<E>,
    trs: &mut merlin::Transcript,
    rng: &mut R,
) -> anyhow::Result<()>
where
    E::ScalarField: FftField,
{
    let (g1_terms, g2_terms) = batch_pairing_for_verify_generalized(
        srs,
        sets,
        hom,
        commitment_msms,
        y_rev,
        phi_y,
        proof,
        trs,
        rng,
    )?;
    let check = E::multi_pairing(g1_terms, g2_terms);
    anyhow::ensure!(PairingOutput::<E>::ZERO == check);
    Ok(())
}

/// Returns (g1_terms, g2_terms) for the pairing check so callers can merge with other checks if needed.
#[allow(non_snake_case)]
pub fn batch_pairing_for_verify_generalized<
    E: Pairing,
    R: RngCore + CryptoRng,
    H: EvalHomomorphism<E::ScalarField>,
>(
    srs: &Srs<E>,
    sets: &[EvaluationSet<E::ScalarField>],
    hom: &H,
    commitment_msms: &[MsmInput<E::G1Affine, E::ScalarField>],
    y_rev: &[Vec<E::ScalarField>],
    phi_y: E::ScalarField,
    proof: &ShplonkedBatchProof<E>,
    trs: &mut merlin::Transcript,
    rng: &mut R,
) -> anyhow::Result<(Vec<E::G1Affine>, Vec<E::G2Affine>)>
where
    E::ScalarField: FftField,
{
    let ShplonkedBatchProof {
        pi_1,
        pi_2: (pi_2_W_prime, pi_2_Y),
        sigma_proof,
        sigma_proof_statement,
    } = proof;
    anyhow::ensure!(
        phi_y == sigma_proof_statement.phi_y,
        "φ(y) does not match proof"
    );

    append_batch_statement_to_transcript::<E>(
        trs,
        sets,
        y_rev,
        phi_y,
        &sigma_proof_statement.com_y_hid,
    );
    let c: E::ScalarField = trs.challenge_scalar();
    trs.append_point(pi_1);
    let x: E::ScalarField = trs.challenge_scalar();

    let n = sets.len();
    anyhow::ensure!(
        commitment_msms.len() == n,
        "commitment count must match evaluation set count"
    );
    for set in sets.iter() {
        anyhow::ensure!(!set.is_empty(), "all evaluation sets S_i must be nonempty");
    }

    let s_union = union_of_evaluation_sets(sets);
    let z_S = zero_poly_S(&s_union);
    let s_per_poly: Vec<Vec<E::ScalarField>> = sets
        .iter()
        .map(|set| set.all_points().cloned().collect())
        .collect();
    let c_powers = powers(c, n);
    let (z_S_val, weights) = compute_weights::<E>(&z_S, &s_per_poly, x, &c_powers);

    let commitment_refs: Vec<&MsmInput<E::G1Affine, E::ScalarField>> =
        commitment_msms.iter().collect();
    let merged = merge_scaled_msm_terms::<E::G1>(&commitment_refs, &weights);

    let msm_pi1 = MsmInput::new(vec![*pi_1], vec![-z_S_val]).expect("MSM pi_1");
    let merged_minus_pi1 = merge_scaled_msm_terms::<E::G1>(&[&merged, &msm_pi1], &[
        E::ScalarField::ONE,
        E::ScalarField::ONE,
    ]);

    let h: usize = sigma_proof.z_yi.iter().map(|v| v.len()).sum();
    let com_y_hom = shplonked_sigma::com_y_hom(&srs.taus_1[..h], srs.xi_1);
    // One weight per polynomial. Lagrange at x: evaluate cached basis polys at x (Horner).
    let (canonical, lagrange_cache) = build_lagrange_cache(&s_per_poly);
    let lagrange_at_x: Vec<Vec<E::ScalarField>> = (0..n)
        .map(|j| {
            let bases = lagrange_cache[canonical[j]].as_ref().unwrap();
            let n_rev = sets[j].rev.len();
            (0..sets[j].hid.len())
                .map(|i| bases[n_rev + i].evaluate(&x))
                .collect()
        })
        .collect();
    let eval_point_commit_hom = shplonked_sigma::EvalPointCommitHom::new(
        srs.taus_1[0],
        srs.xi_1,
        weights.clone(),
        lagrange_at_x,
    );
    let first_tuple_hom = shplonked_sigma::FirstTupleHom::<E> {
        hom1: com_y_hom,
        hom2: eval_point_commit_hom,
        _group: std::marker::PhantomData,
    };
    let sum_hom = shplonked_sigma::SumHom::<E::ScalarField>::default();
    let full_hom = shplonked_sigma::ShplonkedSigmaHom::<E> {
        hom1: first_tuple_hom,
        hom2: sum_hom,
    };

    let public_statement = TupleCodomainShape(
        TupleCodomainShape(
            CodomainShape(sigma_proof_statement.com_y_hid),
            CodomainShape(sigma_proof_statement.C_eval),
        ),
        phi_y,
    );
    let sigma_protocol_proof: Proof<E::ScalarField, shplonked_sigma::ShplonkedSigmaHom<E>> =
        Proof {
            first_proof_item: FirstProofItem::Commitment(TupleCodomainShape(
                TupleCodomainShape(
                    CodomainShape(sigma_proof.r_com_y),
                    CodomainShape(sigma_proof.r_V),
                ),
                sigma_proof.r_y,
            )),
            z: ShplonkedSigmaWitness {
                C_y_hid_randomness: sigma_proof.z_rho,
                evals: sigma_proof.z_yi.clone(), // already { z_i }_i per polynomial
                C_evals_randomness: sigma_proof.z_u,
            },
        };

    let prover_commitment = sigma_protocol_proof
        .prover_commitment()
        .expect("batch verify: expected commitment");
    let c_sigma = crate::sigma_protocol::traits::fiat_shamir_challenge_for_sigma_protocol::<
        _,
        E::ScalarField,
        _,
    >(
        SHPLONKED_SIGMA_DST,
        &full_hom,
        &public_statement,
        prover_commitment,
        &full_hom.dst(),
    );

    anyhow::ensure!(
        hom.apply(y_rev, &sigma_proof.z_yi) == sigma_proof.r_y + c_sigma * phi_y,
        "sigma protocol scalar check (φ(y^rev, z) = r_φ + c·φ(y)) failed"
    );

    let (_, powers_of_beta) = full_hom.hom1.compute_verifier_challenges(
        &public_statement.0,
        &prover_commitment.0,
        SHPLONKED_SIGMA_DST,
        Some(2),
        rng,
    );
    let msm_terms_response = full_hom.hom1.msm_terms(&sigma_protocol_proof.z);
    let hom1_msm_terms = <shplonked_sigma::FirstTupleHom<E> as CurveGroupTrait>::merge_msm_terms(
        msm_terms_response.into_iter().collect::<Vec<_>>(),
        &prover_commitment.0,
        &public_statement.0,
        &powers_of_beta,
        c_sigma,
    );
    // Spec Step 4: deferred G₁ MSM from π_PoK; Step 5a: C_f = ∑_i c^{i-1} Z_{S\S_i}(x)·C_i − Z_S(x)·π_1 − C_eval + c^n·C_PoK.
    let C_PoK = E::G1::msm(hom1_msm_terms.bases(), hom1_msm_terms.scalars())
        .expect("batch verify: C_PoK MSM");
    let c_n = (0..n).fold(E::ScalarField::ONE, |acc, _| acc * c);
    let merged_minus_pi1_pt = E::G1::msm(merged_minus_pi1.bases(), merged_minus_pi1.scalars())
        .expect("batch verify: commitment to f MSM");
    let C_f = merged_minus_pi1_pt - sigma_proof_statement.C_eval.into_group() + C_PoK * c_n;

    // Step 5b: PCS.Verify(vk, x, C_f, 0, π_2) — opening at (x, 0).
    let g1_terms = E::G1::normalize_batch(&[C_f, -pi_2_W_prime.into_group(), -pi_2_Y.into_group()]);
    let g2_terms = vec![
        srs.g_2,
        (srs.tau_2.into_group() - srs.g_2.into_group() * x).into_affine(),
        srs.xi_2,
    ];
    Ok((g1_terms, g2_terms))
}

// ---------------------------------------------------------------------------
// PolynomialCommitmentScheme trait implementation (univariate, single point)
// ---------------------------------------------------------------------------

/// Commitment to a single univariate polynomial (one G1 element).
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct ShplonkedCommitment<E: Pairing>(pub E::G1);

/// Verifier input: MSM representation so it can be merged into one big MSM in verify.
pub type ShplonkedVerifierCommitment<E> =
    MsmInput<<E as Pairing>::G1Affine, <E as Pairing>::ScalarField>;

impl<E: Pairing> From<ShplonkedCommitment<E>> for ShplonkedVerifierCommitment<E> {
    fn from(com: ShplonkedCommitment<E>) -> Self {
        MsmInput::new(vec![com.0.into_affine()], vec![E::ScalarField::ONE])
            .expect("single base and scalar")
    }
}

impl<E> PolynomialCommitmentScheme for Shplonked<E>
where
    E: Pairing,
{
    type Commitment = ShplonkedCommitment<E>;
    type CommitmentKey = Srs<E>;
    type CommitmentNormalised = ShplonkedVerifierCommitment<E>;
    type Polynomial = DensePolynomial<E::ScalarField>;
    type Proof = ShplonkedBatchProof<E>;
    type VerificationKey = Srs<E>;
    type WitnessField = E::ScalarField;

    fn setup<R: rand_core::RngCore + rand_core::CryptoRng>(
        degree_bounds: Vec<usize>,
        rng: &mut R,
    ) -> (Self::CommitmentKey, Self::VerificationKey) {
        // Need at least 5 tau powers: π_2 uses [1]_1,[τ]_1; commitments to q and quotient need degree+1 (e.g. degree 4 → 5).
        let m = degree_bounds
            .iter()
            .map(|&d| d + 1)
            .max()
            .unwrap_or(1)
            .next_power_of_two()
            .max(5);
        let trapdoor = Trapdoor::<E>::rand(rng);
        let (vk_extra, ck) = univariate_hiding_kzg::setup_extra(
            m,
            SrsType::PowersOfTau,
            GroupGenerators::default(),
            trapdoor,
        );
        let taus_1 = match &ck.msm_basis {
            SrsBasis::PowersOfTau { tau_powers } => tau_powers.clone(),
            SrsBasis::Lagrange { .. } => panic!("Shplonked PCS requires PowersOfTau SRS"),
        };
        let srs = Srs {
            taus_1,
            xi_1: ck.xi_1,
            g_2: vk_extra.vk.group_generators.g2,
            tau_2: vk_extra.vk.tau_2,
            xi_2: vk_extra.vk.xi_2,
        };
        (srs.clone(), srs)
    }

    fn commit(
        ck: &Self::CommitmentKey,
        poly: Self::Polynomial,
        r: Option<Self::WitnessField>,
    ) -> Self::Commitment {
        let r = r.expect("Shplonked::commit requires commitment randomness");
        let hom = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
            msm_basis: &ck.taus_1,
            xi_1: ck.xi_1,
        };
        let comm = hom
            .apply(&univariate_hiding_kzg::Witness {
                hiding_randomness: Scalar(r),
                values: Scalar::vec_from_inner(poly.coeffs.clone()),
            })
            .0;
        ShplonkedCommitment(comm)
    }

    fn open<R: RngCore + CryptoRng>(
        ck: &Self::CommitmentKey,
        poly: Self::Polynomial,
        challenge: Vec<Self::WitnessField>,
        r: Option<Self::WitnessField>,
        rng: &mut R,
        trs: &mut merlin::Transcript,
    ) -> Self::Proof {
        let r = r.expect("Shplonked::open requires commitment randomness");
        let point = challenge
            .first()
            .copied()
            .expect("Shplonked univariate open requires one challenge point");
        let sets = vec![EvaluationSet {
            rev: vec![],
            hid: vec![point],
        }];
        let polys = vec![poly];
        let rho_i = vec![r];
        let opening = batch_open_generalized::<E, R, SumEvalHom>(
            ck,
            &sets,
            &polys,
            &rho_i,
            &SumEvalHom,
            trs,
            rng,
        );
        opening.proof
    }

    fn batch_open<R: RngCore + CryptoRng>(
        ck: Self::CommitmentKey,
        polys: Vec<Self::Polynomial>,
        challenge: Vec<Self::WitnessField>,
        rs: Option<Vec<Self::WitnessField>>,
        rng: &mut R,
        trs: &mut merlin::Transcript,
    ) -> Self::Proof {
        let rs = rs.expect("Shplonked::batch_open requires randomness per polynomial");
        let point = challenge
            .first()
            .copied()
            .expect("Shplonked univariate requires one challenge point");
        let eval_points: Vec<E::ScalarField> = (0..polys.len()).map(|_| point).collect();
        let sets: Vec<EvaluationSet<E::ScalarField>> = eval_points
            .iter()
            .map(|&z| EvaluationSet {
                rev: vec![],
                hid: vec![z],
            })
            .collect();
        let opening = batch_open_generalized::<E, R, SumEvalHom>(
            &ck,
            &sets,
            &polys,
            &rs,
            &SumEvalHom,
            trs,
            rng,
        );
        opening.proof
    }

    fn verify(
        vk: &Self::VerificationKey,
        com: impl Into<Self::CommitmentNormalised>,
        challenge: Vec<Self::WitnessField>,
        eval: Self::WitnessField,
        proof: Self::Proof,
        trs: &mut merlin::Transcript,
        _batch: bool,
    ) -> anyhow::Result<()> {
        let point = challenge
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Shplonked verify: expected one challenge point"))?;
        anyhow::ensure!(
            proof.sigma_proof_statement.phi_y == eval,
            "claimed eval does not match opening proof"
        );
        let mut rng = rand::thread_rng();
        let sets = vec![EvaluationSet {
            rev: vec![],
            hid: vec![point],
        }];
        let y_rev: Vec<Vec<E::ScalarField>> = sets.iter().map(|_| vec![]).collect();
        batch_verify_generalized::<E, _, SumEvalHom>(
            vk,
            &sets,
            &SumEvalHom,
            &[com.into()],
            &y_rev,
            eval,
            &proof,
            trs,
            &mut rng,
        )
    }

    fn random_witness<R: rand_core::RngCore + rand_core::CryptoRng>(
        rng: &mut R,
    ) -> Self::WitnessField {
        sample_field_element::<E::ScalarField, _>(rng)
    }

    fn polynomial_from_vec(vec: Vec<Self::WitnessField>) -> Self::Polynomial {
        DensePolynomial::from_coefficients_vec(vec)
    }

    fn evaluate_point(
        poly: &Self::Polynomial,
        point: &Vec<Self::WitnessField>,
    ) -> Self::WitnessField {
        let x = point
            .first()
            .copied()
            .expect("univariate point must have one element");
        poly.evaluate(&x)
    }

    fn scheme_name() -> &'static [u8] {
        b"Shplonked"
    }
}

/// Type marker for the Shplonked PCS (univariate, batch opening support).
pub struct Shplonked<E: Pairing>(core::marker::PhantomData<E>);

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::arkworks::random::sample_field_element;
    use ark_bn254::{Bn254, Fr};
    use ark_poly::Polynomial;
    use rand_core::OsRng;

    #[test]
    fn test_batch_open_verify_generalized_single_point_per_poly() {
        let mut rng = OsRng;
        let (srs, _vk) = Shplonked::<Bn254>::setup(vec![4, 4], &mut rng);

        let polys: Vec<DensePolynomial<Fr>> = vec![
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
        ];
        let points = vec![
            sample_field_element(&mut rng),
            sample_field_element(&mut rng),
        ];
        let sets: Vec<EvaluationSet<Fr>> = points
            .iter()
            .map(|&p| EvaluationSet {
                rev: vec![],
                hid: vec![p],
            })
            .collect();
        let rho_i: Vec<Fr> = (0..polys.len())
            .map(|_| sample_field_element(&mut rng))
            .collect();

        let mut trs = merlin::Transcript::new(b"shplonked_test");
        let opening = batch_open_generalized::<Bn254, _, _>(
            &srs,
            &sets,
            &polys,
            &rho_i,
            &SumEvalHom,
            &mut trs,
            &mut rng,
        );
        assert!(opening.get_evals().iter().all(|v| v.is_empty()));
        assert_eq!(
            opening.get_phi_eval(),
            polys[0].evaluate(&points[0]) + polys[1].evaluate(&points[1])
        );

        let commitments: Vec<ark_bn254::G1Projective> = polys
            .iter()
            .zip(rho_i.iter())
            .map(|(p, &r)| Shplonked::<Bn254>::commit(&srs, p.clone(), Some(r)).0)
            .collect();
        let commitment_msms: Vec<MsmInput<ark_bn254::G1Affine, Fr>> = commitments
            .iter()
            .map(|c| MsmInput::new(vec![c.into_affine()], vec![Fr::ONE]).expect("msm"))
            .collect();

        let mut trs_v = merlin::Transcript::new(b"shplonked_test");
        let ok = batch_verify_generalized::<Bn254, _, _>(
            &srs,
            &sets,
            &SumEvalHom,
            &commitment_msms,
            opening.get_evals(),
            opening.get_phi_eval(),
            &opening.proof,
            &mut trs_v,
            &mut rng,
        );
        // TODO: pairing check can still fail; if so, debug C_f vs commitment_to_f and HKZG formula.
        assert!(ok.is_ok(), "batch verify: {:?}", ok.err());
    }
}
