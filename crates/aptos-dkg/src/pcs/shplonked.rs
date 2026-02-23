// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// ZK-PCS (Shplonked) opening proof types and routines, extracted for use by range proofs.

// WARNING: THIS CODE HAS NOT BEEN PROPERLY VETTED, ONLY USE FOR BENCHMARKING PURPOSES!!!!!

use crate::{
    fiat_shamir::PolynomialCommitmentScheme as _,
    pcs::{
        shplonked_sigma::{self, ShplonkedSigmaWitness},
        traits::PolynomialCommitmentScheme,
        univariate_hiding_kzg::{self, Trapdoor},
    },
    sigma_protocol::{
        homomorphism::{tuple::TupleCodomainShape, Trait as _, TrivialShape as CodomainShape},
        FirstProofItem, Proof, Trait as SigmaTrait,
    },
    Scalar,
};
use aptos_crypto::arkworks::{
    msm::{merge_scaled_msm_terms, MsmInput},
    random::sample_field_element,
    srs::{SrsBasis, SrsType},
    GroupGenerators,
};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    AffineRepr, CurveGroup, VariableBaseMSM,
};
use ark_ff::{Field, One, Zero};
use ark_poly::{
    univariate::{DenseOrSparsePolynomial as DOSPoly, DensePolynomial},
    DenseUVPolynomial, Polynomial,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::{CryptoRng, RngCore};

/// Domain separation tag for the Shplonked opening sigma protocol (Fiat–Shamir context).
pub const SHPLONKED_SIGMA_DST: &[u8; 19] = b"Shplonked_Sigma_Dst";

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Srs<E: Pairing> {
    pub(crate) taus_1: Vec<E::G1Affine>,
    pub(crate) xi_1: E::G1Affine,
    pub(crate) g_2: E::G2Affine,
    pub(crate) tau_2: E::G2Affine,
    pub(crate) xi_2: E::G2Affine,
}

// we will use hiding KZG for now
pub fn zk_pcs_commit<E: Pairing>(
    srs: &Srs<E>,
    f_is: Vec<Vec<E::ScalarField>>,
    r_is: Vec<E::ScalarField>,
) -> Vec<E::G1> {
    assert_eq!(f_is.len(), r_is.len());

    let hom = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
        msm_basis: &srs.taus_1,
        xi_1: srs.xi_1,
    };

    f_is.iter()
        .zip(r_is.iter())
        .map(|(f_i, r_i)| {
            hom.apply(&univariate_hiding_kzg::Witness {
                hiding_randomness: Scalar(*r_i),
                values: Scalar::vec_from_inner(f_i.clone()),
            })
            .0
        })
        .collect()
}

#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Clone, CanonicalDeserialize, Debug)]
struct ZkPcsOpeningSigmaProof<E: Pairing> {
    r_com_y: E::G1Affine,
    r_V: E::G1Affine,
    r_y: E::ScalarField,
    z_yi: Vec<E::ScalarField>,
    z_u: E::ScalarField,
    z_rho: E::ScalarField,
}

/// Statement for the sigma protocol: commitment to evaluations and related values.
#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Clone, CanonicalDeserialize, Debug)]
pub struct ZkPcsOpeningSigmaProofStatement<E: Pairing> {
    pub com_y: E::G1Affine,
    pub V: E::G1Affine,
    pub y_sum: E::ScalarField,
}

#[allow(private_interfaces)]
#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Clone, CanonicalDeserialize, Debug)]
pub struct ZkPcsOpeningProof<E: Pairing> {
    pub(crate) eval_points: Vec<E::ScalarField>,
    pub(crate) W: E::G1Affine,
    pub(crate) W_prime: E::G1Affine,
    pub(crate) Y: E::G1Affine, // extra HKZG term
    pub(crate) sigma_proof: ZkPcsOpeningSigmaProof<E>,
    pub(crate) sigma_proof_statement: ZkPcsOpeningSigmaProofStatement<E>,
}

/// Computes weights alpha_i = gamma^{i-1} * Z_{T\\x_i}(z) for the sigma protocol (eval_points, z, gamma).
#[allow(non_snake_case)]
fn compute_alpha<E: Pairing>(
    eval_points: &[E::ScalarField],
    z: E::ScalarField,
    gamma: E::ScalarField,
) -> Vec<E::ScalarField> {
    let mut z_T = DensePolynomial::from_coefficients_vec(vec![E::ScalarField::ONE]);
    for x in eval_points.iter() {
        let factor = DensePolynomial::from_coefficients_vec(vec![-(*x), E::ScalarField::ONE]);
        z_T = &z_T * &factor;
    }
    let z_T_dos = DOSPoly::from(z_T);
    let mut alphas = Vec::with_capacity(eval_points.len());
    let mut gamma_i = E::ScalarField::ONE;
    for x_i in eval_points.iter() {
        let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![
            -(*x_i),
            E::ScalarField::ONE,
        ]));
        let (z_t_i_poly, remainder) = z_T_dos.divide_with_q_and_r(&divisor).unwrap();
        debug_assert!(remainder.is_zero());
        let z_t_i_val = DensePolynomial::from(z_t_i_poly).evaluate(&z);
        alphas.push(gamma_i * z_t_i_val);
        gamma_i *= gamma;
    }
    alphas
}

/// Opens at the given points; the opening proof includes the sigma proof statement (com_y, V, y_sum).
#[allow(non_snake_case)]
pub fn zk_pcs_open<E: Pairing, R: RngCore + CryptoRng>(
    srs: &Srs<E>,
    _d: u8,
    f_is: Vec<Vec<E::ScalarField>>,
    _commitments: Vec<E::G1>,
    eval_points: Vec<E::ScalarField>,
    evals: Vec<E::ScalarField>,
    rs: Vec<E::ScalarField>,
    trs: &mut merlin::Transcript,
    rng: &mut R,
) -> ZkPcsOpeningProof<E> {
    // Step 1
    assert!(
        srs.taus_1.len() >= 2,
        "Shplonked opening requires SRS with at least 2 tau powers (taus_1[0], taus_1[1]); got {}",
        srs.taus_1.len()
    );
    let hom = univariate_hiding_kzg::CommitmentHomomorphism::<E> {
        msm_basis: &srs.taus_1,
        xi_1: srs.xi_1,
    };
    let rho = sample_field_element(rng);

    let com_y = hom
        .apply(&univariate_hiding_kzg::Witness {
            hiding_randomness: Scalar(rho),
            values: Scalar::vec_from_inner(evals.clone()),
        })
        .0;
    trs.append_point(&com_y.into_affine());

    // Step 2
    let gamma: E::ScalarField = trs.challenge_scalar();

    // Step 3
    // First construct Z_T TODO: optimise this
    let mut z_T = DensePolynomial::from_coefficients_vec(vec![E::ScalarField::ONE]);
    for x in eval_points.iter() {
        let factor = DensePolynomial::from_coefficients_vec(vec![-(*x), E::ScalarField::ONE]);
        z_T = &z_T * &factor;
    }
    let z_T_dos = DOSPoly::from(z_T.clone());

    let f_i_minus_y_is: Vec<DensePolynomial<_>> = f_is
        .iter()
        .zip(evals.iter())
        .map(|(f_i, y_i)| {
            let mut term_poly = DensePolynomial::from_coefficients_vec(f_i.clone());
            term_poly.coeffs[0] -= y_i;
            term_poly
        })
        .collect();

    let z_t_is: Vec<_> = eval_points
        .iter()
        .map(|x_i| {
            let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![
                -(*x_i),
                E::ScalarField::ONE,
            ]));
            let (z_t_i, remainder) = z_T_dos.divide_with_q_and_r(&divisor).unwrap();
            debug_assert!(remainder.is_zero());
            z_t_i
        })
        .collect();

    let mut f_poly = DensePolynomial::zero();
    let mut gamma_i = E::ScalarField::ONE;

    for i in 0..f_i_minus_y_is.len() {
        let term_poly = &f_i_minus_y_is[i];
        let z_t_i = &z_t_is[i];
        let scaled = z_t_i * term_poly * gamma_i;
        f_poly += &scaled;
        gamma_i *= gamma;
    }

    let (h_poly, remainder) = DOSPoly::from(f_poly).divide_with_q_and_r(&z_T_dos).unwrap();
    debug_assert!(remainder.is_zero());

    let s = sample_field_element(rng);
    let W = hom
        .apply(&univariate_hiding_kzg::Witness {
            hiding_randomness: Scalar(s),
            values: Scalar::vec_from_inner(h_poly.coeffs().to_vec()),
        })
        .0;
    trs.append_point(&W.into_affine());

    // Step 4
    let z = trs.challenge_scalar();

    let z_T_val = z_T.evaluate(&z);

    let mut f_z_poly = DensePolynomial::<E::ScalarField>::zero();
    let mut gamma_i = E::ScalarField::ONE;

    for i in 0..f_is.len() {
        let mut term_poly = DensePolynomial::from_coefficients_vec(f_is[i].clone());
        term_poly.coeffs[0] -= evals[i];
        let z_t_i_val = z_t_is[i].evaluate(&z);
        let scaled = term_poly * (gamma_i * z_t_i_val);
        f_z_poly += &scaled;
        gamma_i *= gamma;
    }

    let ZT_h_poly = h_poly.clone() * z_T_val;
    let L_poly = &f_z_poly - &ZT_h_poly;

    let L_dos = DOSPoly::from(L_poly.clone());
    let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![
        -z,
        E::ScalarField::one(),
    ]));
    let (Q_dos, remainder) = L_dos.divide_with_q_and_r(&divisor).unwrap();
    debug_assert!(remainder.is_zero());
    let Q_poly: DensePolynomial<E::ScalarField> = Q_dos.into();

    let t = sample_field_element(rng);
    let W_prime = hom
        .apply(&univariate_hiding_kzg::Witness {
            hiding_randomness: Scalar(t),
            values: Scalar::vec_from_inner(Q_poly.coeffs.clone()),
        })
        .0;
    trs.append_point(&W_prime.into_affine());

    let u: E::ScalarField = sample_field_element(rng);

    let mut sum_y = E::ScalarField::zero();
    let mut gamma_i = E::ScalarField::ONE;

    for (y_i, x_i) in evals.iter().zip(eval_points.iter()) {
        let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![
            -*x_i,
            E::ScalarField::ONE,
        ]));
        let z_T_dos = DOSPoly::from(z_T.clone());
        let (z_t_i_poly, remainder) = z_T_dos.divide_with_q_and_r(&divisor).unwrap();
        debug_assert!(remainder.is_zero());
        let z_t_i_val = DensePolynomial::from(z_t_i_poly).evaluate(&z);
        sum_y += gamma_i * z_t_i_val * (*y_i);
        gamma_i *= gamma;
    }

    let V = srs.xi_1 * u + srs.taus_1[0] * sum_y;

    let mut sum_r = E::ScalarField::zero();
    let mut gamma_i = E::ScalarField::ONE;

    for (r_i, x_i) in rs.iter().zip(eval_points.iter()) {
        let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![
            -*x_i,
            E::ScalarField::ONE,
        ]));
        let z_T_dos = DOSPoly::from(z_T.clone());
        let (z_t_i_poly, remainder) = z_T_dos.divide_with_q_and_r(&divisor).unwrap();
        debug_assert!(remainder.is_zero());
        let z_t_i_val = DensePolynomial::from(z_t_i_poly).evaluate(&z);
        sum_r += gamma_i * z_t_i_val * (*r_i);
        gamma_i *= gamma;
    }

    let y_term = sum_r - z_T_val * s - u + z * t;

    // Y = [1]_1 · y_term − t · [τ]_1 (whitepaper)
    let Y = srs.taus_1[0] * y_term - srs.taus_1[1] * t;

    let y_sum: E::ScalarField = evals.iter().copied().sum();

    let witness = ShplonkedSigmaWitness {
        rho,
        evals: evals.clone(),
        u,
    };
    let alpha = compute_alpha::<E>(&eval_points, z, gamma);
    let com_y_hom = shplonked_sigma::com_y_hom(srs);
    let v_hom = shplonked_sigma::VHom::from_srs(srs, alpha);
    let com_y_v_hom = shplonked_sigma::ComYVHom::<E> {
        hom1: com_y_hom,
        hom2: v_hom,
        _group: std::marker::PhantomData,
    };
    let sum_hom = shplonked_sigma::SumEvalsHom::<E::ScalarField>::default();
    let full_hom = shplonked_sigma::ShplonkedSigmaHom::<E> {
        hom1: com_y_v_hom,
        hom2: sum_hom,
    };

    let statement = TupleCodomainShape(
        TupleCodomainShape(CodomainShape(com_y), CodomainShape(V)),
        y_sum,
    );
    let (sigma_protocol_proof, _) = full_hom.prove(&witness, statement, SHPLONKED_SIGMA_DST, rng);

    let (r_com_y, r_V, r_y) = match &sigma_protocol_proof.first_proof_item {
        FirstProofItem::Commitment(c) => (c.0 .0 .0, c.0 .1 .0, c.1),
        FirstProofItem::Challenge(_) => panic!("expected commitment in sigma proof"),
    };
    let z_rho = sigma_protocol_proof.z.rho;
    let z_yi = sigma_protocol_proof.z.evals;
    let z_u = sigma_protocol_proof.z.u;

    let points_proj = vec![r_com_y.into_group(), r_V.into_group(), V, W, W_prime, Y];
    let affines = E::G1::normalize_batch(&points_proj);
    let [r_com_y, r_V, V, W, W_prime, Y]: [_; 6] = affines.try_into().expect("expected 6 points");
    let sigma_proof = ZkPcsOpeningSigmaProof {
        r_com_y,
        r_V,
        r_y,
        z_yi,
        z_u,
        z_rho,
    };

    let sigma_proof_statement = ZkPcsOpeningSigmaProofStatement {
        com_y: com_y.into_affine(),
        V,
        y_sum,
    };

    let proof = ZkPcsOpeningProof {
        eval_points,
        sigma_proof_statement,
        W,
        W_prime,
        Y,
        sigma_proof,
    };
    proof
}

/// Verifier derives gamma, z and c from the shared transcript (same trs as prover, or
/// a fresh transcript with the same DST and prior content so challenges match).
/// Commitments are given as MSM inputs so they can be combined into one MSM with the opening weights.
#[allow(non_snake_case)]
pub fn zk_pcs_verify<E: Pairing, R: RngCore + CryptoRng>(
    zk_pcs_opening_proof: &ZkPcsOpeningProof<E>,
    commitment_msms: &[MsmInput<E::G1Affine, E::ScalarField>],
    srs: &Srs<E>,
    trs: &mut merlin::Transcript,
    rng: &mut R,
) -> anyhow::Result<()> {
    let ZkPcsOpeningProof {
        eval_points,
        sigma_proof_statement,
        W,
        W_prime,
        Y,
        sigma_proof,
    } = zk_pcs_opening_proof;

    let com_y = sigma_proof_statement.com_y;
    let V = sigma_proof_statement.V;

    trs.append_point(&com_y);

    let gamma: E::ScalarField = trs.challenge_scalar();
    trs.append_point(W);
    let z: E::ScalarField = trs.challenge_scalar();
    trs.append_point(W_prime);

    let mut alphas = Vec::with_capacity(eval_points.len());
    let mut gamma_i = E::ScalarField::ONE;

    let mut z_T = DensePolynomial::from_coefficients_vec(vec![E::ScalarField::ONE]);
    for x in eval_points.iter() {
        let factor = DensePolynomial::from_coefficients_vec(vec![-(*x), E::ScalarField::ONE]);
        z_T = &z_T * &factor;
    }

    for x_i in eval_points.iter() {
        let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![
            -*x_i,
            E::ScalarField::ONE,
        ]));
        let z_T_dos = DOSPoly::from(z_T.clone());
        let (z_t_i_poly, remainder) = z_T_dos.divide_with_q_and_r(&divisor).unwrap();
        debug_assert!(remainder.is_zero());
        let z_t_i_val = DensePolynomial::from(z_t_i_poly).evaluate(&z);
        alphas.push(gamma_i * z_t_i_val);
        gamma_i *= gamma;
    }

    let commitment_refs: Vec<&MsmInput<E::G1Affine, E::ScalarField>> =
        commitment_msms.iter().collect();
    let merged = merge_scaled_msm_terms::<E::G1>(&commitment_refs, &alphas);
    let sum_com = E::G1::msm(merged.bases(), merged.scalars())
        .expect("Shplonked verify: merged commitment MSM");

    let z_T_val = z_T.evaluate(&z);

    // Paper: F := Σ_i γ^{i-1}·Z_{T∖x_i}(z)·com_i − Z_T(z)·W − V
    // Check: e(F + z·W', [1]_2) = e(W', [τ]_2) · e(Y, [ξ]_2)
    // So e(F+z·W', g_2) · e(−W', τ_2) · e(−Y, ξ_2) = identity
    let F = sum_com - (*W) * z_T_val - V;
    let g1_terms = vec![
        (F + (*W_prime) * z).into_affine(),
        (-(*W_prime).into_group()).into_affine(),
        (-(*Y).into_group()).into_affine(),
    ];
    let g2_terms = vec![srs.g_2, srs.tau_2, srs.xi_2];

    let result = E::multi_pairing(g1_terms, g2_terms);
    if PairingOutput::<E>::zero() != result {
        return Err(anyhow::anyhow!("Expected zero during multi-pairing check"));
    }

    let alpha = compute_alpha::<E>(&eval_points, z, gamma);
    let com_y_hom = shplonked_sigma::com_y_hom(srs);
    let v_hom = shplonked_sigma::VHom::from_srs(srs, alpha);
    let com_y_v_hom = shplonked_sigma::ComYVHom::<E> {
        hom1: com_y_hom,
        hom2: v_hom,
        _group: std::marker::PhantomData,
    };
    let sum_hom = shplonked_sigma::SumEvalsHom::<E::ScalarField>::default();
    let full_hom = shplonked_sigma::ShplonkedSigmaHom::<E> {
        hom1: com_y_v_hom,
        hom2: sum_hom,
    };

    let public_statement = TupleCodomainShape(
        TupleCodomainShape(
            CodomainShape(sigma_proof_statement.com_y),
            CodomainShape(sigma_proof_statement.V),
        ),
        sigma_proof_statement.y_sum,
    );

    let sigma_protocol_proof = Proof {
        first_proof_item: FirstProofItem::Commitment(TupleCodomainShape(
            TupleCodomainShape(
                CodomainShape(sigma_proof.r_com_y),
                CodomainShape(sigma_proof.r_V),
            ),
            sigma_proof.r_y,
        )),
        z: ShplonkedSigmaWitness {
            rho: sigma_proof.z_rho,
            evals: sigma_proof.z_yi.clone(),
            u: sigma_proof.z_u,
        },
    };

    full_hom.verify(
        &public_statement,
        &sigma_protocol_proof,
        SHPLONKED_SIGMA_DST,
        Some((2, 0)), // ((com_y, V), y_sum): 2 components in first tuple, 0 MSMs in second
        rng,
    )
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
    type Polynomial = DensePolynomial<E::ScalarField>;
    type Proof = ZkPcsOpeningProof<E>;
    type VerificationKey = Srs<E>;
    type VerifierCommitment = ShplonkedVerifierCommitment<E>;
    type WitnessField = E::ScalarField;

    fn setup<R: rand_core::RngCore + rand_core::CryptoRng>(
        degree_bounds: Vec<usize>,
        rng: &mut R,
    ) -> (Self::CommitmentKey, Self::VerificationKey) {
        // Opening proof needs taus_1[0] and taus_1[1] (Y = [1]_1·y_term − t·[τ]_1), so require m >= 2.
        let m = degree_bounds
            .iter()
            .map(|&d| d + 1)
            .max()
            .unwrap_or(1)
            .next_power_of_two()
            .max(2);
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
        let coeffs = poly.coeffs.clone();
        let comms = zk_pcs_commit(ck, vec![coeffs], vec![r]);
        ShplonkedCommitment(comms[0])
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
        let coeffs = poly.coeffs.clone();
        let eval = poly.evaluate(&point);
        let com = Self::commit(
            ck,
            DensePolynomial::from_coefficients_vec(coeffs.clone()),
            Some(r),
        );
        let commitments = vec![com.0];
        let opening = zk_pcs_open(
            ck,
            0, // degree not used for single poly
            vec![coeffs],
            commitments,
            vec![point],
            vec![eval],
            vec![r],
            trs,
            rng,
        );
        opening
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
        let f_is: Vec<Vec<E::ScalarField>> = polys.iter().map(|p| p.coeffs.clone()).collect();
        let evals: Vec<E::ScalarField> = polys.iter().map(|p| p.evaluate(&point)).collect();
        let commitments: Vec<E::G1> = f_is
            .iter()
            .zip(rs.iter())
            .map(|(coeffs, &r)| zk_pcs_commit(&ck, vec![coeffs.clone()], vec![r])[0])
            .collect();
        let opening = zk_pcs_open(
            &ck,
            0,
            f_is,
            commitments,
            vec![point; polys.len()],
            evals,
            rs,
            trs,
            rng,
        );
        opening
    }

    fn verify(
        vk: &Self::VerificationKey,
        com: impl Into<Self::VerifierCommitment>,
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
            proof.eval_points.len() == 1 && proof.eval_points[0] == point,
            "challenge point does not match opening proof"
        );
        anyhow::ensure!(
            proof.sigma_proof_statement.y_sum == eval,
            "claimed eval does not match opening proof"
        );
        let mut rng = rand::thread_rng();
        let com_msm = com.into();
        zk_pcs_verify(&proof, &[com_msm], vk, trs, &mut rng)
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
