// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// ZK-PCS (Shplonked) opening proof types and routines, extracted for use by range proofs.

use crate::{
    fiat_shamir::PolynomialCommitmentScheme as _, pcs::univariate_hiding_kzg,
    sigma_protocol::homomorphism::Trait as _, Scalar,
};
use aptos_crypto::arkworks::random::{sample_field_element, sample_field_elements};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    CurveGroup, VariableBaseMSM,
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

#[derive(CanonicalSerialize, Clone, CanonicalDeserialize)]
struct ZkPcsOpeningSigmaProof<E: Pairing> {
    r_com_y: E::G1Affine,
    r_V: E::G1Affine,
    r_y: E::ScalarField,
    z_yi: Vec<E::ScalarField>,
    z_u: E::ScalarField,
    z_rho: E::ScalarField,
}

#[derive(CanonicalSerialize, Clone, CanonicalDeserialize)]
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

    let Y = srs.taus_1[0] * y_term - srs.xi_1 * u;

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
    let z_u = r_u + c * u;
    let z_rho = r_rho + c * rho;

    let mut points_proj = vec![r_com_y, r_V, V, W, W_prime, Y];
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

    ZkPcsOpeningProof {
        eval_points,
        gamma,
        z,
        y_sum,
        V,
        W,
        W_prime,
        Y,
        sigma_proof,
    }
}

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

    let _gamma: E::ScalarField = merlin::Transcript::new(b"gamma").challenge_scalar();

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

    let _z: E::ScalarField = merlin::Transcript::new(b"zed").challenge_scalar();

    let z_T_val = z_T.evaluate(&z);

    let F = sum_com - (*W) * z_T_val - (*V);

    let g1_terms = vec![(-F - (*W_prime) * z).into_affine(), *W_prime, *Y];

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
        z_u: _z_u,
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

    scalars.push(-beta);
    scalars.push(-c * beta);

    bases.push(srs.taus_1[0]);
    bases.push(srs.xi_1);
    bases.push(*r_V);
    bases.push(*V);

    E::G1::msm(&bases, &scalars);

    let lhs_y: E::ScalarField = z_yi.iter().copied().sum();
    let rhs_y = *r_y + *y_sum * c;

    assert_eq!(lhs_y, rhs_y, "sigma proof y sum check failed");

    Ok(())
}
