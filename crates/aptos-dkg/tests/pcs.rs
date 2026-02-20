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
use ark_ff::{One, Zero};
use rand::thread_rng;

const PCS_BATCH_DST: &[u8] = b"pcs_batch_open_test";

fn test_pcs_setup_commit_open_verify<PCS: PolynomialCommitmentScheme>() {
    let mut rng = thread_rng();
    let num_point_dims = PCS::default_num_point_dims_for_tests();
    let degree_bounds = PCS::degree_bounds_for_test_point_dims(num_point_dims);

    let (ck, vk) = PCS::setup(degree_bounds, &mut rng);

    // Use 2^num_point_dims so polynomial and challenge dimensions match (Zeromorph multilinear).
    let poly_len = 1u32 << num_point_dims;
    let poly = random_poly::<PCS, _>(&mut rng, poly_len, 32);
    let r = PCS::random_witness(&mut rng);
    let com = PCS::commit(&ck, poly.clone(), Some(r));

    let challenge_pt = random_point::<PCS, _>(&mut rng, num_point_dims);
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

    // Verifier uses a fresh transcript with the same DST so it derives the same gamma, z, c.
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

fn test_pcs_polynomial_from_vec_evaluate_point<PCS: PolynomialCommitmentScheme>()
where
    PCS::WitnessField: From<u64>,
{
    let mut rng = thread_rng();
    let num_point_dims = PCS::default_num_point_dims_for_tests();

    let values: Vec<PCS::WitnessField> =
        (0..16).map(|i| PCS::WitnessField::from(i as u64)).collect();
    let poly = PCS::polynomial_from_vec(values);

    let point = random_point::<PCS, _>(&mut rng, num_point_dims);
    let eval = PCS::evaluate_point(&poly, &point);

    let poly2 = poly.clone();
    let eval2 = PCS::evaluate_point(&poly2, &point);
    assert_eq!(eval, eval2, "evaluate_point should be deterministic");
}

mod zeromorph {
    use super::*;
    use aptos_dkg::pcs::zeromorph::{Zeromorph, ZeromorphCommitment};
    use ark_ec::pairing::Pairing;

    #[test]
    fn zeromorph_bn254_setup_commit_open_verify() {
        test_pcs_setup_commit_open_verify::<Zeromorph<Bn254>>();
    }

    #[test]
    fn zeromorph_bn254_polynomial_from_vec_evaluate_point() {
        test_pcs_polynomial_from_vec_evaluate_point::<Zeromorph<Bn254>>();
    }

    #[test]
    fn zeromorph_bn254_batch_open_verify() {
        let mut rng = thread_rng();
        let degree_bounds = vec![1, 1, 1, 1];
        let num_point_dims = 4u32;

        let (ck, vk) = Zeromorph::<Bn254>::setup(degree_bounds, &mut rng);

        let poly_len = 1u32 << num_point_dims; // 16 so multilinear has 4 vars
        let polys: Vec<_> = (0..3)
            .map(|_| random_poly::<Zeromorph<Bn254>, _>(&mut rng, poly_len, 32))
            .collect();
        let rs: Vec<_> = (0..polys.len())
            .map(|_| Zeromorph::<Bn254>::random_witness(&mut rng))
            .collect();
        let challenge = random_point::<Zeromorph<Bn254>, _>(&mut rng, num_point_dims);

        let gamma: <Bn254 as Pairing>::ScalarField = first_transcript_challenge(PCS_BATCH_DST);

        let gammas: Vec<_> = (0..polys.len())
            .scan(<Bn254 as Pairing>::ScalarField::one(), |acc, _| {
                let g = *acc;
                *acc *= gamma;
                Some(g)
            })
            .collect();

        let combined_eval = polys
            .iter()
            .zip(gammas.iter())
            .map(|(p, g)| Zeromorph::<Bn254>::evaluate_point(p, &challenge) * *g)
            .fold(<Bn254 as Pairing>::ScalarField::zero(), |a, b| a + b);

        let commitments: Vec<ZeromorphCommitment<Bn254>> = polys
            .iter()
            .zip(rs.iter())
            .map(|(p, r)| {
                <Zeromorph<Bn254> as PolynomialCommitmentScheme>::commit(&ck, p.clone(), Some(*r))
            })
            .collect();

        let combined_g1 = commitments
            .iter()
            .zip(gammas.iter())
            .map(|(c, g)| (c.as_inner().into_affine(), *g))
            .fold(<Bn254 as Pairing>::G1::zero(), |acc, (c, g)| acc + c * g);
        let combined_com = ZeromorphCommitment::from_g1(combined_g1);

        let mut trs = merlin::Transcript::new(PCS_BATCH_DST);
        let proof = Zeromorph::<Bn254>::batch_open(
            ck,
            polys,
            challenge.clone(),
            Some(rs),
            &mut rng,
            &mut trs,
        );

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
        test_pcs_setup_commit_open_verify::<Shplonked<Bn254>>();
    }

    #[test]
    fn shplonked_bn254_polynomial_from_vec_evaluate_point() {
        test_pcs_polynomial_from_vec_evaluate_point::<Shplonked<Bn254>>();
    }

    #[test]
    fn shplonked_bn254_batch_open_verify() {
        let mut rng = thread_rng();
        let degree_bounds = vec![15];
        let num_point_dims = 1u32;

        let (ck, vk) = Shplonked::<Bn254>::setup(degree_bounds, &mut rng);

        let polys: Vec<_> = (0..3)
            .map(|_| random_poly::<Shplonked<Bn254>, _>(&mut rng, 8, 32))
            .collect();
        let rs: Vec<_> = (0..polys.len())
            .map(|_| Shplonked::<Bn254>::random_witness(&mut rng))
            .collect();
        let challenge = random_point::<Shplonked<Bn254>, _>(&mut rng, num_point_dims);

        let commitments: Vec<_> = polys
            .iter()
            .zip(rs.iter())
            .map(|(p, r)| Shplonked::<Bn254>::commit(&ck, p.clone(), Some(*r)))
            .collect();
        let commitment_msms: Vec<_> = commitments.iter().map(|c| c.clone().into()).collect();

        let mut trs_prover = merlin::Transcript::new(PCS_BATCH_DST);
        let proof = Shplonked::<Bn254>::batch_open(
            ck,
            polys,
            challenge.clone(),
            Some(rs),
            &mut rng,
            &mut trs_prover,
        );

        let mut trs_verifier = merlin::Transcript::new(PCS_BATCH_DST);
        zk_pcs_verify(&proof, &commitment_msms, &vk, &mut trs_verifier, &mut rng)
            .expect("batch verify should succeed");
    }
}
