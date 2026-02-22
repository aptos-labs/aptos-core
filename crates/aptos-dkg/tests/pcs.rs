// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Generic tests for any type implementing [PolynomialCommitmentScheme].
//! Instantiated for Zeromorph and Shplonked below.

use aptos_dkg::pcs::{
    shplonked::Shplonked,
    traits::{first_transcript_challenge, random_point, random_poly, PolynomialCommitmentScheme},
};
use ark_bn254::Bn254;
use ark_ec::CurveGroup;
use ark_ff::Zero;
use rand::thread_rng;

const PCS_BATCH_DST: &[u8] = b"pcs_batch_open_test";

/// Parameters for running generic PCS tests (degree bounds, point dimension, polynomial length).
struct PcsTestParams {
    degree_bounds: Vec<usize>,
    num_point_dims: u32,
    poly_len: u32,
}

/// Setup for univariate PCS tests: one variable, degree bound d, poly length = number of coefficients.
fn univariate_setup(degree_bound: usize, poly_len: u32) -> PcsTestParams {
    PcsTestParams {
        degree_bounds: vec![degree_bound],
        num_point_dims: 1,
        poly_len,
    }
}

/// Setup for multilinear PCS tests: n variables, degree 1 per dim, 2^n coefficients.
fn multilinear_setup(num_vars: u32) -> PcsTestParams {
    PcsTestParams {
        degree_bounds: vec![1; num_vars as usize],
        num_point_dims: num_vars,
        poly_len: 1 << num_vars,
    }
}

fn test_pcs_setup_commit_open_verify<PCS: PolynomialCommitmentScheme>(params: PcsTestParams) {
    let mut rng = thread_rng();
    let (ck, vk) = PCS::setup(params.degree_bounds.clone(), &mut rng);

    let poly = random_poly::<PCS, _>(&mut rng, params.poly_len, None);
    let r = PCS::random_witness(&mut rng);
    let com = PCS::commit(&ck, poly.clone(), Some(r));

    let challenge_pt = random_point::<PCS, _>(&mut rng, params.num_point_dims);
    let eval = PCS::evaluate_point(&poly, &challenge_pt);

    let mut trs_prover = merlin::Transcript::new(PCS::transcript_dst_for_single_open());
    let proof = PCS::open(
        &ck,
        poly,
        challenge_pt.clone(),
        Some(r),
        &mut rng,
        &mut trs_prover,
    );

    // Verifier uses a fresh transcript with the same DST so it hopefully derives the same challenges
    let mut trs_verifier = merlin::Transcript::new(PCS::transcript_dst_for_single_open());
    PCS::verify(
        &vk,
        com,
        challenge_pt,
        eval,
        proof,
        &mut trs_verifier,
        false,
    )
    .expect("verify should succeed");
}

// Test if point evaluation is deterministic
fn test_pcs_polynomial_from_vec_evaluate_point<PCS: PolynomialCommitmentScheme>(
    params: PcsTestParams,
) {
    let mut rng = thread_rng();

    let values: Vec<PCS::WitnessField> = (0..params.poly_len)
        .map(|_| PCS::random_witness(&mut rng))
        .collect();
    let poly = PCS::polynomial_from_vec(values);

    let point = random_point::<PCS, _>(&mut rng, params.num_point_dims);
    let eval = PCS::evaluate_point(&poly, &point);

    let poly2 = poly.clone();
    let eval2 = PCS::evaluate_point(&poly2, &point);
    assert_eq!(eval, eval2, "evaluate_point should be deterministic");
}

mod zeromorph {
    use super::*;
    use aptos_crypto::utils::powers;
    use aptos_dkg::pcs::zeromorph::{Zeromorph, ZeromorphCommitment};
    use ark_ec::pairing::Pairing;

    #[test]
    fn zeromorph_bn254_setup_commit_open_verify() {
        test_pcs_setup_commit_open_verify::<Zeromorph<Bn254>>(multilinear_setup(8));
    }

    #[test]
    fn zeromorph_bn254_polynomial_from_vec_evaluate_point() {
        test_pcs_polynomial_from_vec_evaluate_point::<Zeromorph<Bn254>>(multilinear_setup(4));
    }

    #[test]
    fn zeromorph_bn254_batch_open_verify() {
        let mut rng = thread_rng();
        let params = multilinear_setup(4);

        let (ck, vk) = Zeromorph::<Bn254>::setup(params.degree_bounds, &mut rng);

        // batch of 4 polynomials
        let polys: Vec<_> = (0..3)
            .map(|_| random_poly::<Zeromorph<Bn254>, _>(&mut rng, params.poly_len, None))
            .collect();
        let rs: Vec<_> = (0..polys.len())
            .map(|_| Zeromorph::<Bn254>::random_witness(&mut rng))
            .collect();
        let challenge = random_point::<Zeromorph<Bn254>, _>(&mut rng, params.num_point_dims);

        let mut trs_prover = merlin::Transcript::new(PCS_BATCH_DST);
        let proof = Zeromorph::<Bn254>::batch_open(
            ck.clone(),
            polys.clone(),
            challenge.clone(),
            Some(rs.clone()),
            &mut rng,
            &mut trs_prover,
        );

        // TODO: make the verifier stuff more generic
        let gamma: <Bn254 as Pairing>::ScalarField = first_transcript_challenge(PCS_BATCH_DST);

        let gammas = powers(gamma, polys.len());

        // TODO: combine the polynomials and evaluate the combined polynomial at the challenge point
        let combined_eval = polys
            .iter()
            .zip(gammas.iter())
            .map(|(p, g)| Zeromorph::<Bn254>::evaluate_point(p, &challenge) * *g)
            .fold(<Bn254 as Pairing>::ScalarField::zero(), |a, b| a + b);

        // Commit to the polynomials
        let commitments: Vec<ZeromorphCommitment<Bn254>> = polys
            .iter()
            .zip(rs.iter())
            .map(|(p, r)| {
                <Zeromorph<Bn254> as PolynomialCommitmentScheme>::commit(
                    &ck.clone(),
                    p.clone(),
                    Some(*r),
                )
            })
            .collect();

        // Combine the commitments into a single commitment
        let combined_g1 = commitments
            .iter()
            .zip(gammas.iter())
            .map(|(c, g)| (c.as_inner().into_affine(), *g))
            .fold(<Bn254 as Pairing>::G1::zero(), |acc, (c, g)| acc + c * g);
        let combined_com = ZeromorphCommitment::from_g1(combined_g1);

        let mut trs_verify = merlin::Transcript::new(PCS_BATCH_DST);
        Zeromorph::<Bn254>::verify(
            &vk,
            &combined_com,
            &challenge,
            &combined_eval,
            &proof,
            &mut trs_verify,
            true,
        )
        .expect("batch verify should succeed");
    }
}

mod shplonked {
    use super::*;
    use aptos_dkg::pcs::shplonked::zk_pcs_verify;

    #[test]
    fn shplonked_bn254_setup_commit_open_verify() {
        test_pcs_setup_commit_open_verify::<Shplonked<Bn254>>(univariate_setup(15, 16));
    }

    #[test]
    fn shplonked_bn254_polynomial_from_vec_evaluate_point() {
        test_pcs_polynomial_from_vec_evaluate_point::<Shplonked<Bn254>>(univariate_setup(15, 16));
    }

    #[test]
    fn shplonked_bn254_batch_open_verify() {
        let mut rng = thread_rng();
        let params = univariate_setup(15, 8);

        let (ck, vk) = Shplonked::<Bn254>::setup(params.degree_bounds, &mut rng);

        let polys: Vec<_> = (0..3)
            .map(|_| random_poly::<Shplonked<Bn254>, _>(&mut rng, params.poly_len, Some(32)))
            .collect();
        let rs: Vec<_> = (0..polys.len())
            .map(|_| Shplonked::<Bn254>::random_witness(&mut rng))
            .collect();
        let challenge = random_point::<Shplonked<Bn254>, _>(&mut rng, params.num_point_dims);

        let mut trs_prover = merlin::Transcript::new(PCS_BATCH_DST);
        let proof = Shplonked::<Bn254>::batch_open(
            ck.clone(),
            polys.clone(),
            challenge.clone(),
            Some(rs.clone()),
            &mut rng,
            &mut trs_prover,
        );

        let commitments: Vec<_> = polys
            .iter()
            .zip(rs.iter())
            .map(|(p, r)| Shplonked::<Bn254>::commit(&ck, p.clone(), Some(*r)))
            .collect();
        let commitment_msms: Vec<_> = commitments.iter().map(|c| c.clone().into()).collect();

        let mut trs_verifier = merlin::Transcript::new(PCS_BATCH_DST);
        zk_pcs_verify(&proof, &commitment_msms, &vk, &mut trs_verifier, &mut rng)
            .expect("batch verify should succeed");
    }
}
