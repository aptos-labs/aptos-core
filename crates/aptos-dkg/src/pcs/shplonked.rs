// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// ZK-PCS (Shplonked) opening proof types and routines, extracted for use by range proofs.

use crate::{
    fiat_shamir::PolynomialCommitmentScheme as _,
    pcs::{
        traits::PolynomialCommitmentScheme,
        univariate_hiding_kzg::{self, Trapdoor},
    },
    sigma_protocol::homomorphism::Trait as _,
    Scalar,
};
use aptos_crypto::arkworks::{
    random::{sample_field_element, sample_field_elements},
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

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Srs<E: Pairing> {
    pub(crate) taus_1: Vec<E::G1Affine>,
    pub(crate) xi_1: E::G1Affine,
    pub(crate) g_2: E::G2Affine,
    pub(crate) tau_2: E::G2Affine,
    pub(crate) xi_2: E::G2Affine,
}

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

#[allow(private_interfaces)]
#[allow(non_snake_case)]
#[derive(CanonicalSerialize, Clone, CanonicalDeserialize, Debug)]
pub struct ZkPcsOpeningProof<E: Pairing> {
    pub(crate) eval_points: Vec<E::ScalarField>,
    pub(crate) gamma: E::ScalarField,
    pub(crate) z: E::ScalarField,
    pub(crate) y_sum: E::ScalarField,
    pub(crate) V: E::G1Affine,
    pub(crate) W: E::G1Affine,
    pub(crate) W_prime: E::G1Affine,
    pub(crate) Y: E::G1Affine,
    pub(crate) sigma_proof: ZkPcsOpeningSigmaProof<E>,
}

/// Opens at the given points and returns both the opening proof and the commitment to evaluations (com_y).
#[allow(non_snake_case)]
pub fn zk_pcs_open_with_com_y<E: Pairing, R: RngCore + CryptoRng>(
    srs: &Srs<E>,
    _d: u8,
    f_is: Vec<Vec<E::ScalarField>>,
    _commitments: Vec<E::G1>,
    eval_points: Vec<E::ScalarField>,
    evals: Vec<E::ScalarField>,
    rs: Vec<E::ScalarField>,
    trs: &mut merlin::Transcript,
    rng: &mut R,
) -> (ZkPcsOpeningProof<E>, E::G1Affine) {
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

    let gamma = trs.challenge_scalar();

    let mut z_T = DensePolynomial::from_coefficients_vec(vec![E::ScalarField::ONE]);

    for x in eval_points.iter() {
        let factor = DensePolynomial::from_coefficients_vec(vec![-(*x), E::ScalarField::ONE]);
        z_T = &z_T * &factor;
    }

    let f_i_minus_y_is: Vec<DensePolynomial<_>> = f_is
        .iter()
        .zip(evals.iter())
        .map(|(f_i, y_i)| {
            let mut term_poly = DensePolynomial::from_coefficients_vec(f_i.clone());
            term_poly.coeffs[0] -= y_i;
            term_poly
        })
        .collect();

    let z_T_dos = DOSPoly::from(z_T.clone());

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

    let r_yi: Vec<E::ScalarField> = sample_field_elements(f_is.len(), rng);
    let r_u: E::ScalarField = sample_field_element(rng);
    let r_rho = sample_field_element(rng);

    let mut scalars = vec![r_rho];
    scalars.extend(r_yi.iter().copied());
    let mut bases = vec![srs.xi_1];
    bases.extend(srs.taus_1.iter().cloned());
    let r_com_y = E::G1::msm_unchecked(&bases, &scalars);

    let mut r_sum_y = E::ScalarField::zero();
    let mut gamma_i = E::ScalarField::ONE;
    for (r_i, x_i) in r_yi.iter().zip(eval_points.iter()) {
        let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![
            -*x_i,
            E::ScalarField::ONE,
        ]));
        let z_T_dos = DOSPoly::from(z_T.clone());
        let (z_t_i_poly, remainder) = z_T_dos.divide_with_q_and_r(&divisor).unwrap();
        debug_assert!(remainder.is_zero());
        let z_t_i_val = DensePolynomial::from(z_t_i_poly).evaluate(&z);
        r_sum_y += gamma_i * z_t_i_val * (*r_i);
        gamma_i *= gamma;
    }

    let r_V = srs.taus_1[0] * r_sum_y - srs.xi_1 * r_u;

    let r_y: E::ScalarField = r_yi.iter().copied().sum();

    let c: E::ScalarField = merlin::Transcript::new(b"verifier_challenge").challenge_scalar();

    let mut z_yi = Vec::with_capacity(f_is.len());
    for (r_i, w_i) in r_yi.iter().zip(evals.iter()) {
        z_yi.push(*r_i + c * w_i);
    }
    // r_V uses -r_u*[ξ]_1, V uses +u*[ξ]_1, so response for [ξ] must be -r_u + c*u
    let z_u = c * u - r_u;
    let z_rho = r_rho + c * rho;

    let points_proj = vec![r_com_y, r_V, V, W, W_prime, Y];
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

    let y_sum: E::ScalarField = evals.iter().copied().sum();

    let proof = ZkPcsOpeningProof {
        eval_points,
        gamma,
        z,
        y_sum,
        V,
        W,
        W_prime,
        Y,
        sigma_proof,
    };
    (proof, com_y.into_affine())
}

pub fn zk_pcs_open<E: Pairing, R: RngCore + CryptoRng>(
    srs: &Srs<E>,
    d: u8,
    f_is: Vec<Vec<E::ScalarField>>,
    commitments: Vec<E::G1>,
    eval_points: Vec<E::ScalarField>,
    evals: Vec<E::ScalarField>,
    rs: Vec<E::ScalarField>,
    trs: &mut merlin::Transcript,
    rng: &mut R,
) -> ZkPcsOpeningProof<E> {
    zk_pcs_open_with_com_y(srs, d, f_is, commitments, eval_points, evals, rs, trs, rng).0
}

/// Verifier uses gamma and z from the proof (prover stored them there after reading from transcript).
#[allow(non_snake_case)]
pub fn zk_pcs_verify<E: Pairing, R: RngCore + CryptoRng>(
    zk_pcs_opening_proof: &ZkPcsOpeningProof<E>,
    commitments: &[E::G1Affine],
    com_y: E::G1Affine,
    srs: &Srs<E>,
    rng: &mut R,
) -> anyhow::Result<()> {
    let ZkPcsOpeningProof {
        eval_points,
        gamma,
        z,
        y_sum,
        V,
        W,
        W_prime,
        Y,
        sigma_proof,
    } = zk_pcs_opening_proof;

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

    let sum_com = E::G1::msm_unchecked(commitments, &alphas);

    let z_T_val = z_T.evaluate(&z);

    // Paper: F := Σ_i γ^{i-1}·Z_{T∖x_i}(z)·com_i − Z_T(z)·W − V
    // Check: e(F + z·W', [1]_2) = e(W', [τ]_2) · e(Y, [ξ]_2)
    // So e(F+z·W', g_2) · e(−W', τ_2) · e(−Y, ξ_2) = identity
    let F = sum_com - (*W) * z_T_val - (*V);
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

    let c: E::ScalarField = merlin::Transcript::new(b"verifier_challenge").challenge_scalar();

    let ZkPcsOpeningSigmaProof {
        r_com_y,
        r_V,
        r_y,
        z_yi,
        z_u,
        z_rho,
    } = sigma_proof;

    let mut scalars = vec![*z_rho];
    scalars.extend(z_yi.iter().copied());
    scalars.push(-E::ScalarField::one());
    scalars.push(-c);

    let mut bases = vec![srs.xi_1];
    bases.extend(srs.taus_1.iter().take(z_yi.len()).copied());
    bases.push(*r_com_y);
    bases.push(com_y);

    let beta: E::ScalarField = sample_field_element(rng);

    let mut sum_y = E::ScalarField::zero();
    let mut gamma_i = E::ScalarField::ONE;

    for (s_i, x_i) in z_yi.iter().zip(eval_points.iter()) {
        let divisor = DOSPoly::from(DensePolynomial::from_coefficients_vec(vec![
            -*x_i,
            E::ScalarField::ONE,
        ]));
        let z_T_dos = DOSPoly::from(z_T.clone());
        let (z_t_i_poly, remainder) = z_T_dos.divide_with_q_and_r(&divisor).unwrap();
        debug_assert!(remainder.is_zero());
        let z_t_i_val = DensePolynomial::from(z_t_i_poly).evaluate(&z);
        sum_y += gamma_i * z_t_i_val * (*s_i);
        gamma_i *= gamma;
    }

    // V relation: sum_y*[1]_1 + z_u*xi_1 - r_V - c*V = 0; batch with beta
    scalars.push(beta * sum_y);
    scalars.push(beta * (*z_u));
    scalars.push(-beta);
    scalars.push(-c * beta);

    bases.push(srs.taus_1[0]);
    bases.push(srs.xi_1);
    bases.push(*r_V);
    bases.push(*V);

    // Sigma protocol (group homomorphism) check: response MSM must equal identity
    let sigma_msm = E::G1::msm(&bases, &scalars)
        .map_err(|e| anyhow::anyhow!("Sigma proof MSM failed: {:?}", e))?;
    if sigma_msm != E::G1::zero() {
        return Err(anyhow::anyhow!(
            "Sigma proof group homomorphism check failed (expected identity)"
        ));
    }

    let lhs_y: E::ScalarField = z_yi.iter().copied().sum();
    let rhs_y = *r_y + *y_sum * c;

    assert_eq!(lhs_y, rhs_y, "sigma proof y sum check failed");

    Ok(())
}

// ---------------------------------------------------------------------------
// PolynomialCommitmentScheme trait implementation (univariate, single point)
// ---------------------------------------------------------------------------

/// Commitment to a single univariate polynomial (one G1 element).
#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct ShplonkedCommitment<E: Pairing>(pub E::G1);

/// Proof for the PCS trait: opening proof plus commitment to the evaluation (com_y).
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ShplonkedPcsProof<E: Pairing> {
    pub opening: ZkPcsOpeningProof<E>,
    pub com_y: E::G1Affine,
}

impl<E> PolynomialCommitmentScheme for Shplonked<E>
where
    E: Pairing,
{
    type Commitment = ShplonkedCommitment<E>;
    type CommitmentKey = Srs<E>;
    type Polynomial = DensePolynomial<E::ScalarField>;
    type Proof = ShplonkedPcsProof<E>;
    type VerificationKey = Srs<E>;
    type WitnessField = E::ScalarField;

    fn setup<R: rand_core::RngCore + rand_core::CryptoRng>(
        degree_bounds: Vec<usize>,
        rng: &mut R,
    ) -> (Self::CommitmentKey, Self::VerificationKey) {
        let m = degree_bounds
            .iter()
            .map(|&d| d + 1)
            .max()
            .unwrap_or(1)
            .next_power_of_two();
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
        let (opening, com_y) = zk_pcs_open_with_com_y(
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
        ShplonkedPcsProof { opening, com_y }
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
        let (opening, com_y) = zk_pcs_open_with_com_y(
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
        ShplonkedPcsProof { opening, com_y }
    }

    fn verify(
        vk: &Self::VerificationKey,
        com: Self::Commitment,
        challenge: Vec<Self::WitnessField>,
        eval: Self::WitnessField,
        proof: Self::Proof,
        _trs: &mut merlin::Transcript,
        _batch_dst: Option<&'static [u8]>,
    ) -> anyhow::Result<()> {
        let point = challenge
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Shplonked verify: expected one challenge point"))?;
        anyhow::ensure!(
            proof.opening.eval_points.len() == 1 && proof.opening.eval_points[0] == point,
            "challenge point does not match opening proof"
        );
        anyhow::ensure!(
            proof.opening.y_sum == eval,
            "claimed eval does not match opening proof"
        );
        let mut rng = rand::thread_rng();
        let commitments = vec![com.0.into_affine()];
        zk_pcs_verify(&proof.opening, &commitments, proof.com_y, vk, &mut rng)
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
