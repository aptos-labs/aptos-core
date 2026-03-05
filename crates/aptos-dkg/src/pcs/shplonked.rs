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
//! C_eval = g·τ_0 + ρ_eval·ξ_1 with g = ∑_i weight_i·f̃_i(x) (all evals, rev + hid).

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
            fixed_base_msms::Trait as FixedBaseMsmsTrait, tuple::TupleCodomainShape,
            Trait as HomTrait, TrivialShape as CodomainShape,
        },
        traits::fiat_shamir_challenge_for_sigma_protocol,
        CurveGroupTrait, Proof, Trait as SigmaTrait,
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
use std::fmt::Debug;

/// Domain separation tag for the Shplonked opening sigma protocol (Fiat–Shamir context).
pub const SHPLONKED_SIGMA_DST: &[u8; 19] = b"Shplonked_Sigma_Dst";

/// Type marker for the Shplonked PCS (univariate, batch opening support).
pub struct Shplonked<E: Pairing>(core::marker::PhantomData<E>);

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

/// Domain for the evaluation homomorphism φ: (y_rev, y_hid) with one vector of evals per polynomial.
/// Used with [`HomTrait`](crate::sigma_protocol::homomorphism::Trait): φ is any homomorphism with
/// `Domain = EvalPair<F>`, `Codomain = F`, `CodomainNormalized = F`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalPair<F> {
    /// Revealed evaluations per polynomial: { y_i^rev }_i.
    pub y_rev: Vec<Vec<F>>,
    /// Hidden evaluations per polynomial: { y_i^hid }_i.
    pub y_hid: Vec<Vec<F>>,
}

/// Default: φ(y) = ∑_j y_j^hid (sum of all hidden evaluations).
/// Implements the sigma_protocol homomorphism trait with domain [`EvalPair`].
#[derive(Clone, Debug, Default, CanonicalSerialize)]
pub struct SumEvalHom<F>(core::marker::PhantomData<F>);

impl<F: Field> HomTrait for SumEvalHom<F> {
    type Codomain = F;
    type CodomainNormalized = F;
    type Domain = EvalPair<F>;

    fn apply(&self, pair: &Self::Domain) -> Self::Codomain {
        pair.y_hid.iter().flatten().fold(F::zero(), |a, &b| a + b)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        value
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

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Srs<E: Pairing> {
    pub(crate) taus_1: Vec<E::G1Affine>,
    pub(crate) xi_1: E::G1Affine,
    pub(crate) g_2: E::G2Affine,
    pub(crate) tau_2: E::G2Affine,
    pub(crate) xi_2: E::G2Affine,
}

/// Type of the sigma protocol statement: (com_y_hid, C_eval, φ(y)).
type ShplonkedSigmaStatement<E> =
    <shplonked_sigma::ShplonkedSigmaHom<'static, E> as HomTrait>::CodomainNormalized;

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
    /// π_PoK: sigma protocol proof (knowledge of y^hid). Statement is (com_y_hid, C_eval, φ(y)).
    pub sigma_proof: Proof<E::ScalarField, shplonked_sigma::ShplonkedSigmaHom<'static, E>>,
    /// Sigma protocol statement (com_y_hid, C_eval, φ(y)).
    pub sigma_proof_statement: ShplonkedSigmaStatement<E>,
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
        self.proof.sigma_proof_statement.1
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

/// Single label for the whole batch statement so prover and verifier hash the same bytes.
//const SHPLONKED_FS_STATEMENT_DST: &[u8] = b"shplonked-fs-statement-dst";

/// Appends the batch-open statement to the transcript for Fiat–Shamir (Step 1b).
/// Uses a single canonical blob so prover and verifier derive the same c and x.
fn append_batch_statement_to_transcript<E: Pairing>(
    trs: &mut merlin::Transcript,
    sets: &[EvaluationSet<E::ScalarField>],
    y_rev: &[Vec<E::ScalarField>],
    phi_y: E::ScalarField,
    c_y_hid: &E::G1Affine,
) {
    for set in sets {
        trs.append_evaluation_set::<E::ScalarField>(set);
    }
    for y_i_rev in y_rev {
        trs.append_evaluation_points(y_i_rev);
    }
    trs.append_homomorphism_image(&phi_y);
    trs.append_point(c_y_hid);
    //trs.append_message(SHPLONKED_FS_STATEMENT_DST, &buf);
}

/// Computes g_rev = ∑_j weight_j · (∑_{i ∈ rev} L_{j,i}(x) · y_j^rev[i]) from weights, Lagrange cache, and y_rev.
#[allow(non_snake_case)]
fn compute_g_rev<E: Pairing>(
    n: usize,
    sets: &[EvaluationSet<E::ScalarField>],
    weights: &[E::ScalarField],
    canonical: &[usize],
    lagrange_cache: &[Option<Vec<DensePolynomial<E::ScalarField>>>],
    x: E::ScalarField,
    y_rev: &[Vec<E::ScalarField>],
) -> E::ScalarField {
    (0..n)
        .map(|j| {
            let bases = lagrange_cache[canonical[j]].as_ref().unwrap();
            let n_rev = sets[j].rev.len();
            let rev_part: E::ScalarField = (0..n_rev)
                .map(|i| bases[i].evaluate(&x) * y_rev[j][i])
                .sum();
            weights[j] * rev_part
        })
        .sum()
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
    H: HomTrait<
            Domain = EvalPair<E::ScalarField>,
            Codomain = E::ScalarField,
            CodomainNormalized = E::ScalarField,
        > + Clone,
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
    let phi_y = hom.apply(&EvalPair {
        y_rev: y_rev_per_poly.clone(),
        y_hid: y_hid_per_poly.clone(),
    });

    let c_y_hid_randomness = sample_field_element(rng);
    let com_y_hom = shplonked_sigma::com_y_hom::<E>(&srs.taus_1[..y_hid_flat.len()], srs.xi_1);
    let com_y_hid = com_y_hom
        .apply(&shplonked_sigma::ShplonkedSigmaWitness {
            C_y_hid_randomness: c_y_hid_randomness,
            hidden_evals: y_hid_per_poly.clone(),
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

    let g_at_x: E::ScalarField = (0..n)
        .map(|i| weights[i] * tilde_f_is[i].evaluate(&x))
        .sum();

    // Step 4c: C_eval = g·τ_0 + ρ_eval·ξ_1 where g = ∑_j weight_j·f̃_j(x) (all evals, rev + hid).
    // Compute Lagrange-at-x for hidden indices (for sigma) and for revealed (for g_rev).
    let lagrange_at_x_hid: Vec<Vec<E::ScalarField>> = (0..n)
        .map(|j| {
            let bases = lagrange_cache[canonical[j]].as_ref().unwrap();
            let n_rev = sets[j].rev.len();
            (0..sets[j].hid.len())
                .map(|i| bases[n_rev + i].evaluate(&x))
                .collect()
        })
        .collect();
    let g_rev_at_x = compute_g_rev::<E>(
        n,
        sets,
        &weights,
        &canonical,
        &lagrange_cache,
        x,
        &y_rev_per_poly,
    );
    let g_hid_at_x: E::ScalarField = (0..n)
        .map(|j| {
            weights[j]
                * lagrange_at_x_hid[j]
                    .iter()
                    .zip(y_hid_per_poly[j].iter())
                    .map(|(&l, &y)| l * y)
                    .sum::<E::ScalarField>()
        })
        .sum();
    debug_assert_eq!(g_at_x, g_rev_at_x + g_hid_at_x, "g = g_rev + g_hid");

    let eval_point_commit_hom = shplonked_sigma::EvalPointCommitHom::new(
        srs.taus_1[0],
        srs.xi_1,
        weights.clone(),
        lagrange_at_x_hid,
    );
    let witness_for_C_eval_hid = shplonked_sigma::ShplonkedSigmaWitness {
        C_y_hid_randomness: E::ScalarField::zero(),
        hidden_evals: y_hid_per_poly.clone(),
        C_evals_randomness: rho_eval,
    };
    let C_eval_hid_proj: E::G1 = eval_point_commit_hom.apply(&witness_for_C_eval_hid).0;

    // Step 5a: f = ∑_i c^{i-1} Z_{S\S_i}(x) f_i − Z_S(x) q − g.
    let mut f_poly = DensePolynomial::zero();
    for i in 0..n {
        f_poly += &(&f_is[i] * weights[i]);
    }
    f_poly -= &(&q_poly * z_S_val);
    f_poly -= &DensePolynomial::from_coefficients_vec(vec![g_at_x]);

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

    // Step 5d: sigma proof proves (com_y_hid, C_eval_hid) with C_eval_hid = g_hid·τ_0 + ρ·ξ_1.
    let witness = ShplonkedSigmaWitness {
        C_y_hid_randomness: c_y_hid_randomness,
        hidden_evals: y_hid_per_poly.clone(),
        C_evals_randomness: rho_eval,
    };
    let com_y_eval_hom = shplonked_sigma::FirstTupleHom::<E> {
        hom1: com_y_hom,
        hom2: eval_point_commit_hom,
        _group: std::marker::PhantomData,
    };
    // Use the caller's homomorphism φ so the sigma proof proves φ(y_rev, y_hid) = phi_y (not just sum).
    let sum_hom = shplonked_sigma::EvalHomLifted::<E::ScalarField, H> {
        y_rev: y_rev_per_poly.clone(),
        hom: hom.clone(),
    };
    let full_hom = shplonked_sigma::ShplonkedSigmaHomWithEval::<E, H> {
        hom1: com_y_eval_hom,
        hom2: sum_hom,
    };
    let statement_proj = TupleCodomainShape(
        TupleCodomainShape(
            CodomainShape(com_y_hid.into()),
            CodomainShape(C_eval_hid_proj),
        ),
        phi_y,
    );
    let (sigma_protocol_proof, sigma_statement) =
        full_hom.prove(&witness, statement_proj, SHPLONKED_SIGMA_DST, rng);
    let sigma_proof =
        sigma_protocol_proof.change_lifetime::<shplonked_sigma::ShplonkedSigmaHom<'static, E>>();

    // π_2 from PCS.Open: pi_2 = quotient commitment, pi_2_extra = hiding compensation.
    let pi_2 = opening.pi_1.0.into_affine();
    let pi_2_extra = opening.pi_2.into_affine();

    let proof = ShplonkedBatchProof {
        pi_1,
        pi_2: (pi_2, pi_2_extra),
        sigma_proof,
        sigma_proof_statement: sigma_statement,
    };
    ShplonkedBatchOpening {
        evals: y_rev_per_poly,
        proof,
    }
}

/// Generalized batch verify per spec: PCS.BatchVerify(vk, {S_i}, φ, {C_i}; { y_i^rev }_i, φ(y), π) → {0,1}.
/// Commitments C_i may be given as MSM representations (they are expanded into the equation).
/// TODO: Isn't this method already part of the trait?
#[allow(non_snake_case)]
pub fn batch_verify_generalized<
    E: Pairing,
    R: RngCore + CryptoRng,
    H: HomTrait<
            Domain = EvalPair<E::ScalarField>,
            Codomain = E::ScalarField,
            CodomainNormalized = E::ScalarField,
        > + Clone,
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
) -> anyhow::Result<()> {
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
    H: HomTrait<
            Domain = EvalPair<E::ScalarField>,
            Codomain = E::ScalarField,
            CodomainNormalized = E::ScalarField,
        > + Clone,
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
) -> anyhow::Result<(Vec<E::G1Affine>, Vec<E::G2Affine>)> {
    let ShplonkedBatchProof {
        pi_1,
        pi_2: (pi_2_W_prime, pi_2_Y),
        sigma_proof,
        sigma_proof_statement,
    } = proof;
    anyhow::ensure!(
        phi_y == sigma_proof_statement.1,
        "φ(y) does not match proof"
    );

    let (com_y_hid, c_eval_hid) = (
        &sigma_proof_statement.0 .0 .0,
        &sigma_proof_statement.0 .1 .0,
    );
    append_batch_statement_to_transcript::<E>(trs, sets, y_rev, phi_y, com_y_hid);
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

    let h: usize = sigma_proof
        .z
        .hidden_evals
        .iter()
        .map(|v: &Vec<E::ScalarField>| v.len())
        .sum();
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
    let g_rev_at_x = compute_g_rev::<E>(n, sets, &weights, &canonical, &lagrange_cache, x, y_rev);
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
    // Use the same φ as the prover so the Fiat–Shamir challenge c_sigma matches.
    let sum_hom = shplonked_sigma::EvalHomLifted::<E::ScalarField, H> {
        y_rev: y_rev.to_vec(),
        hom: hom.clone(),
    };
    let full_hom = shplonked_sigma::ShplonkedSigmaHomWithEval::<E, H> {
        hom1: first_tuple_hom,
        hom2: sum_hom,
    };

    let statement_curve_for_merge =
        TupleCodomainShape(CodomainShape(*com_y_hid), CodomainShape(*c_eval_hid));

    let prover_commitment = sigma_proof
        .prover_commitment()
        .expect("batch verify: expected commitment");
    let c_sigma = fiat_shamir_challenge_for_sigma_protocol::<_, E::ScalarField, _>(
        SHPLONKED_SIGMA_DST,
        &full_hom,
        sigma_proof_statement,
        prover_commitment,
        &full_hom.dst(),
    );

    let r_sum_ys = prover_commitment.1;
    full_hom
        .hom2
        .verify_with_challenge(&phi_y, &r_sum_ys, c_sigma, &sigma_proof.z, None, rng)?;

    let (_, powers_of_beta) = full_hom.hom1.compute_verifier_challenges(
        &statement_curve_for_merge,
        &prover_commitment.0,
        SHPLONKED_SIGMA_DST,
        Some(2),
        rng,
    );
    let msm_terms_response = full_hom.hom1.msm_terms(&sigma_proof.z);
    let hom1_msm_terms = <shplonked_sigma::FirstTupleHom<E> as CurveGroupTrait>::merge_msm_terms(
        msm_terms_response.into_iter().collect::<Vec<_>>(),
        &prover_commitment.0,
        &statement_curve_for_merge,
        &powers_of_beta,
        c_sigma,
    );
    // C_eval = C_eval_hid + g_rev·τ_0 for the batch pairing check.
    let c_eval = (c_eval_hid.into_group() + srs.taus_1[0].into_group() * g_rev_at_x).into_affine();
    // Spec Step 4: deferred G₁ MSM from π_PoK; Step 5a: C_f = ∑_i c^{i-1} Z_{S\S_i}(x)·C_i − Z_S(x)·π_1 − C_eval + c^n·C_PoK.
    let C_PoK = E::G1::msm(hom1_msm_terms.bases(), hom1_msm_terms.scalars())
        .expect("batch verify: C_PoK MSM");
    let c_n = (0..n).fold(E::ScalarField::ONE, |acc, _| acc * c);
    let merged_minus_pi1_pt = E::G1::msm(merged_minus_pi1.bases(), merged_minus_pi1.scalars())
        .expect("batch verify: commitment to f MSM");
    let C_f = merged_minus_pi1_pt - c_eval.into_group() + C_PoK * c_n;

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
    // This is not ideal, but we need some of the SRS in order to verify the sigma protocol proof.
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
        let opening = batch_open_generalized::<E, R, SumEvalHom<E::ScalarField>>(
            ck,
            &sets,
            &polys,
            &rho_i,
            &SumEvalHom::<E::ScalarField>::default(),
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
        let opening = batch_open_generalized::<E, R, SumEvalHom<E::ScalarField>>(
            &ck,
            &sets,
            &polys,
            &rs,
            &SumEvalHom::<E::ScalarField>::default(),
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
        let phi_y: E::ScalarField = proof.sigma_proof_statement.1;
        anyhow::ensure!(eval == phi_y, "claimed eval does not match opening proof");
        let mut rng = rand::thread_rng();
        let sets = vec![EvaluationSet {
            rev: vec![],
            hid: vec![point],
        }];
        let y_rev: Vec<Vec<E::ScalarField>> = sets.iter().map(|_| vec![]).collect();
        batch_verify_generalized::<E, _, SumEvalHom<E::ScalarField>>(
            vk,
            &sets,
            &SumEvalHom::<E::ScalarField>::default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::arkworks::random::sample_field_element;
    use ark_bn254::{Bn254, Fr};
    use ark_poly::Polynomial;
    use rand_core::OsRng;

    /// Minimal batch open/verify: two polynomials, one or two hidden points per set.
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
        // First set: single point; second set: two points (so we don't only test singleton sets)
        let p0 = sample_field_element(&mut rng);
        let p10 = sample_field_element(&mut rng);
        let p11 = sample_field_element(&mut rng);
        let sets: Vec<EvaluationSet<Fr>> = vec![
            EvaluationSet {
                rev: vec![],
                hid: vec![p0],
            },
            EvaluationSet {
                rev: vec![],
                hid: vec![p10, p11],
            },
        ];
        let rho_i: Vec<Fr> = (0..polys.len())
            .map(|_| sample_field_element(&mut rng))
            .collect();

        let mut trs = merlin::Transcript::new(b"shplonked_test");
        let opening = batch_open_generalized::<Bn254, _, _>(
            &srs,
            &sets,
            &polys,
            &rho_i,
            &SumEvalHom::<Fr>::default(),
            &mut trs,
            &mut rng,
        );
        assert!(opening.get_evals().iter().all(|v| v.is_empty()));
        let expected_phi =
            polys[0].evaluate(&p0) + polys[1].evaluate(&p10) + polys[1].evaluate(&p11);
        assert_eq!(opening.get_phi_eval(), expected_phi);

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
            &SumEvalHom::<Fr>::default(),
            &commitment_msms,
            opening.get_evals(),
            opening.get_phi_eval(),
            &opening.proof,
            &mut trs_v,
            &mut rng,
        );
        assert!(ok.is_ok(), "batch verify: {:?}", ok.err());
    }

    /// General batch open/verify test: more polynomials, multiple hidden points per set,
    /// and varying set sizes (all hidden points).
    #[test]
    fn test_batch_open_verify_generalized_multi_point_per_poly() {
        let mut rng = OsRng;
        let (srs, _vk) = Shplonked::<Bn254>::setup(vec![8, 8, 8, 8], &mut rng);

        // Four polynomials with different degrees
        let polys: Vec<DensePolynomial<Fr>> = vec![
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
        ];

        // Evaluation sets: all hidden, but with 1, 2, 3, and 2 points per poly respectively
        let sets: Vec<EvaluationSet<Fr>> = vec![
            EvaluationSet {
                rev: vec![],
                hid: vec![sample_field_element(&mut rng)],
            },
            EvaluationSet {
                rev: vec![],
                hid: vec![
                    sample_field_element(&mut rng),
                    sample_field_element(&mut rng),
                ],
            },
            EvaluationSet {
                rev: vec![],
                hid: vec![
                    sample_field_element(&mut rng),
                    sample_field_element(&mut rng),
                    sample_field_element(&mut rng),
                ],
            },
            EvaluationSet {
                rev: vec![],
                hid: vec![
                    sample_field_element(&mut rng),
                    sample_field_element(&mut rng),
                ],
            },
        ];

        let rho_i: Vec<Fr> = (0..polys.len())
            .map(|_| sample_field_element(&mut rng))
            .collect();

        let mut trs = merlin::Transcript::new(b"shplonked_multi_point_test");
        let opening = batch_open_generalized::<Bn254, _, _>(
            &srs,
            &sets,
            &polys,
            &rho_i,
            &SumEvalHom::<Fr>::default(),
            &mut trs,
            &mut rng,
        );

        assert!(opening.get_evals().iter().all(|v| v.is_empty()));

        // SumEvalHom: φ(y) = sum of all hidden evals
        let expected_phi_y: Fr = sets
            .iter()
            .zip(polys.iter())
            .map(|(set, poly)| set.hid.iter().map(|p| poly.evaluate(p)).sum::<Fr>())
            .sum();
        assert_eq!(
            opening.get_phi_eval(),
            expected_phi_y,
            "φ(y) = sum of hidden evals"
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

        let mut trs_v = merlin::Transcript::new(b"shplonked_multi_point_test");
        let ok = batch_verify_generalized::<Bn254, _, _>(
            &srs,
            &sets,
            &SumEvalHom::<Fr>::default(),
            &commitment_msms,
            opening.get_evals(),
            opening.get_phi_eval(),
            &opening.proof,
            &mut trs_v,
            &mut rng,
        );
        assert!(ok.is_ok(), "batch verify: {:?}", ok.err());
    }

    /// Batch open/verify with mixed revealed and hidden evaluation sets (C_eval uses all evals).
    #[test]
    fn test_batch_open_verify_generalized_mixed_rev_hid() {
        let mut rng = OsRng;
        let (srs, _vk) = Shplonked::<Bn254>::setup(vec![8, 8, 8, 8], &mut rng);

        let polys: Vec<DensePolynomial<Fr>> = vec![
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
            DensePolynomial::from_coefficients_vec(vec![
                sample_field_element(&mut rng),
                sample_field_element(&mut rng),
            ]),
        ];

        let p00 = sample_field_element(&mut rng);
        let p01 = sample_field_element(&mut rng);
        let p02 = sample_field_element(&mut rng);
        let p10 = sample_field_element(&mut rng);
        let p11 = sample_field_element(&mut rng);
        let p20 = sample_field_element(&mut rng);
        let p21 = sample_field_element(&mut rng);
        let p22 = sample_field_element(&mut rng);
        let p30 = sample_field_element(&mut rng);
        let p31 = sample_field_element(&mut rng);

        let sets: Vec<EvaluationSet<Fr>> = vec![
            EvaluationSet {
                rev: vec![p00, p01],
                hid: vec![p02],
            },
            EvaluationSet {
                rev: vec![],
                hid: vec![p10, p11],
            },
            EvaluationSet {
                rev: vec![p20],
                hid: vec![p21, p22],
            },
            EvaluationSet {
                rev: vec![p30, p31],
                hid: vec![],
            },
        ];

        let rho_i: Vec<Fr> = (0..polys.len())
            .map(|_| sample_field_element(&mut rng))
            .collect();

        let mut trs = merlin::Transcript::new(b"shplonked_mixed_rev_hid_test");
        let opening = batch_open_generalized::<Bn254, _, _>(
            &srs,
            &sets,
            &polys,
            &rho_i,
            &SumEvalHom::<Fr>::default(),
            &mut trs,
            &mut rng,
        );

        let expected_y_rev: Vec<Vec<Fr>> = sets
            .iter()
            .zip(polys.iter())
            .map(|(set, poly)| set.rev.iter().map(|p| poly.evaluate(p)).collect())
            .collect();
        assert_eq!(opening.get_evals(), &expected_y_rev[..]);

        let expected_phi_y: Fr = polys[0].evaluate(&p02)
            + polys[1].evaluate(&p10)
            + polys[1].evaluate(&p11)
            + polys[2].evaluate(&p21)
            + polys[2].evaluate(&p22);
        assert_eq!(opening.get_phi_eval(), expected_phi_y);

        let commitments: Vec<ark_bn254::G1Projective> = polys
            .iter()
            .zip(rho_i.iter())
            .map(|(p, &r)| Shplonked::<Bn254>::commit(&srs, p.clone(), Some(r)).0)
            .collect();
        let commitment_msms: Vec<MsmInput<ark_bn254::G1Affine, Fr>> = commitments
            .iter()
            .map(|c| MsmInput::new(vec![c.into_affine()], vec![Fr::ONE]).expect("msm"))
            .collect();

        let mut trs_v = merlin::Transcript::new(b"shplonked_mixed_rev_hid_test");
        let ok = batch_verify_generalized::<Bn254, _, _>(
            &srs,
            &sets,
            &SumEvalHom::<Fr>::default(),
            &commitment_msms,
            opening.get_evals(),
            opening.get_phi_eval(),
            &opening.proof,
            &mut trs_v,
            &mut rng,
        );
        assert!(
            ok.is_ok(),
            "batch verify with mixed rev/hid: {:?}",
            ok.err()
        );
    }
}
