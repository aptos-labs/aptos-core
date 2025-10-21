// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use aptos_dkg::range_proofs::traits::BatchedRangeProof;
use aptos_dkg::{
    range_proofs::{
        dekart_univariate::Proof as UnivariateDeKART,
        dekart_univariate_v2::Proof as UnivariateDeKARTv2,
    },
    utils::test_utils,
};
use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::pairing::Pairing;
use ark_std::rand::thread_rng;

#[cfg(test)]
fn assert_range_proof_correctness<E: Pairing, B: BatchedRangeProof<E>>(
    setup: &RangeProofUniversalSetup<E, B>,
    n: usize,
    ell: usize,
) {
    let mut rng = thread_rng();
    let RangeProofUniversalSetup { pk, vk } = setup;
    let (values, comm, r) =
        test_utils::range_proof_random_instance::<_, B, _>(pk, n, ell, &mut rng);
    println!("setup finished, prove starting for n={}, ell={}", n, ell);

    let mut fs_t = merlin::Transcript::new(B::DST);
    let proof = B::prove(pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);

    let mut fs_t = merlin::Transcript::new(B::DST);
    proof.verify(vk, n, ell, &comm, &mut fs_t).unwrap();

    // === Serialize to memory ===
    let encoded = {
        let mut v = Vec::new();
        proof
            .serialize_compressed(&mut v)
            .expect("proof serialization should succeed");
        v
    };
    println!(
        "Serialized proof size (n={}, ell={}): {} bytes (expected for blstrs: {} bytes)",
        n,
        ell,
        encoded.len(),
        2 * 8 + 48 + (48 + 96) * ell // Can get rid of the 2 * 8 here by turning the Vecs in `proof` into tuples
    );

    // === Round-trip deserialization ===
    let decoded = B::deserialize_compressed(&*encoded).expect("Deserialization failed");

    // Verify still succeeds
    let mut fs_t = merlin::Transcript::new(B::DST);
    decoded.verify(vk, n, ell, &comm, &mut fs_t).unwrap();

    println!(
        "Serialization round-trip test passed for n={}, ell={}",
        n, ell
    );

    // Make invalid
    let mut invalid_proof = decoded.clone();
    invalid_proof.maul();
    let mut fs_t = merlin::Transcript::new(B::DST);
    assert!(invalid_proof.verify(vk, n, ell, &comm, &mut fs_t).is_err());
}

#[cfg(test)]
const TEST_CASES: &[(usize, usize)] = &[
    // (n, \ell)
//    (3, 16),
    (7, 16),
//    (4, 16),
//    (8, 16),
//    (16, 3),
//    (16, 4),
//    (16, 7),
//    (16, 8),
//    (16, 16),
    // (255, 16),  Commented out to improve test speed
    // (255, 32),
    // (512, 32),
    // (1024, 32),
    // (2047, 32),
];

#[cfg(test)]
/// A **reusable** setup structure.
struct RangeProofUniversalSetup<E: Pairing, B: BatchedRangeProof<E>> {
    pk: B::ProverKey,
    vk: B::VerificationKey,
}

#[cfg(test)]
/// Generate a fixed setup for a single curve
fn make_single_curve_setup<E, B>(n: usize, ell: usize) -> RangeProofUniversalSetup<E, B>
where
    E: Pairing,
    B: BatchedRangeProof<E>,
{
    let mut rng = thread_rng();
    let (pk, vk) = B::setup(n, ell, &mut rng);
    RangeProofUniversalSetup { pk, vk }
}

#[cfg(test)]
fn assert_correctness_for_range_proof_and_curve<E, B>()
where
    E: Pairing,
    B: BatchedRangeProof<E>,
{
    let setups = make_single_curve_setup::<E, B>(7, 16);
    for &(n, ell) in TEST_CASES {
        assert_range_proof_correctness::<E, B>(&setups, n, ell);
    }
}

#[cfg(test)]
#[test]
fn assert_correctness_of_all_range_proofs() {
//    assert_correctness_for_range_proof_and_curve::<Bn254, UnivariateDeKART<Bn254>>();
//   assert_correctness_for_range_proof_and_curve::<Bls12_381, UnivariateDeKART<Bls12_381>>();

    assert_correctness_for_range_proof_and_curve::<Bn254, UnivariateDeKARTv2<Bn254>>();
    assert_correctness_for_range_proof_and_curve::<Bls12_381, UnivariateDeKARTv2<Bls12_381>>();
}
