// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::arkworks::GroupGenerators;
use aptos_dkg::{
    range_proofs::{
        dekart_multivariate::Proof as DekartMultivariate,
        dekart_univariate::Proof as UnivariateDeKART,
        dekart_univariate_v2::Proof as UnivariateDeKARTv2,
        traits::BatchedRangeProof,
    },
    utils::test_utils,
};
use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::thread_rng;
use std::fmt::Debug;

#[cfg(test)]
fn assert_range_proof_correctness<E: Pairing, B: BatchedRangeProof<E>>(
    setup: &RangeProofUniversalSetup<E, B>,
    n: usize,
    ell: u8,
) {
    let mut rng = rand::thread_rng();
    let RangeProofUniversalSetup { pk, vk } = setup;
    let (values, comm, r) =
        test_utils::range_proof_random_instance::<_, B, _>(pk, n, ell, &mut rng);
    println!("setup finished, prove starting for n={}, ell={}", n, ell);

    let proof = B::prove(pk, &values, ell, &comm, &r, &mut rng);
    proof.verify(vk, n, ell, &comm, &mut rng).unwrap();

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
        2 * 8 + 48 + (48 + 96) * ell as usize // Can get rid of the 2 * 8 here by turning the Vecs in `proof` into tuples
    );

    // === Round-trip deserialization ===
    let decoded = B::deserialize_compressed(&*encoded).expect("Deserialization failed");

    // Verify still succeeds
    decoded.verify(vk, n, ell, &comm, &mut rng).unwrap();

    println!(
        "Serialization round-trip test passed for n={}, ell={}",
        n, ell
    );

    // Make invalid
    let mut invalid_proof = decoded.clone();
    invalid_proof.maul();
    assert!(invalid_proof.verify(vk, n, ell, &comm, &mut rng).is_err());
}

#[cfg(test)]
fn assert_keys_serialization<E: Pairing, B: BatchedRangeProof<E>>(
    setup: &RangeProofUniversalSetup<E, B>,
) where
    B::ProverKey: CanonicalSerialize + CanonicalDeserialize + Eq + Debug,
    B::VerificationKey: CanonicalDeserialize + Eq + Debug,
{
    let RangeProofUniversalSetup { pk, vk } = setup;

    // === Prover key serialization/deserialization ===
    let pk_encoded = {
        let mut v = Vec::new();
        pk.serialize_compressed(&mut v)
            .expect("Prover key serialization should succeed");
        v
    };
    println!("Serialized pk size: {} bytes", pk_encoded.len());

    let pk_decoded = B::ProverKey::deserialize_compressed(&*pk_encoded)
        .expect("Prover key deserialization should succeed");
    assert_eq!(pk, &pk_decoded, "Round-trip pk failed");

    // === Verifier key serialization/deserialization ===
    let vk_encoded = {
        let mut v = Vec::new();
        vk.serialize_compressed(&mut v)
            .expect("Verifier key serialization should succeed");
        v
    };
    println!("Serialized vk size: {} bytes", vk_encoded.len());

    let vk_decoded = B::VerificationKey::deserialize_compressed(&*vk_encoded)
        .expect("Verifier key deserialization should succeed");
    assert_eq!(vk, &vk_decoded, "Round-trip vk failed");

    println!("Prover and Verifier key serialization round-trip passed.");
}

#[cfg(test)]
const TEST_CASES: &[(usize, u8)] = &[
    // (n, \ell)
    (1, 16),
    (3, 16),
    (7, 16),
    (4, 16),
    (8, 16),
    (16, 3),
    (16, 4),
    (16, 7),
    (16, 8),
    (16, 16),
];

#[cfg(test)]
/// A **reusable** setup structure.
struct RangeProofUniversalSetup<E: Pairing, B: BatchedRangeProof<E>> {
    pk: B::ProverKey,
    vk: B::VerificationKey,
}

#[cfg(test)]
/// Generate a fixed setup for a single curve
fn make_single_curve_setup<E, B>(n: usize, ell: u8) -> RangeProofUniversalSetup<E, B>
where
    E: Pairing,
    B: BatchedRangeProof<E>,
{
    let mut rng = thread_rng();
    let group_generators = GroupGenerators::default();
    let (pk, vk) = B::setup(n, ell, group_generators, &mut rng);
    RangeProofUniversalSetup { pk, vk }
}

#[cfg(test)]
fn assert_correctness_for_range_proof_and_curve<E, B>()
where
    E: Pairing,
    B: BatchedRangeProof<E>,
{
    let setups = make_single_curve_setup::<E, B>(31, 16);
    for &(n, ell) in TEST_CASES {
        assert_range_proof_correctness::<E, B>(&setups, n, ell);
    }
}

#[cfg(test)]
fn assert_correctness_and_serialization_for_range_proof_and_curve<E, B>()
where
    E: Pairing,
    B: BatchedRangeProof<E>,
    B::ProverKey: CanonicalSerialize + CanonicalDeserialize + Eq + Debug,
    B::VerificationKey: CanonicalDeserialize + Eq + Debug,
{
    let setups = make_single_curve_setup::<E, B>(31, 16);
    for &(n, ell) in TEST_CASES {
        assert_range_proof_correctness::<E, B>(&setups, n, ell);
        assert_keys_serialization::<E, B>(&setups);
    }
}

#[cfg(test)]
#[test]
fn assert_correctness_of_all_range_proofs() {
    assert_correctness_for_range_proof_and_curve::<Bn254, UnivariateDeKART<Bn254>>();
    assert_correctness_for_range_proof_and_curve::<Bls12_381, UnivariateDeKART<Bls12_381>>();

    assert_correctness_and_serialization_for_range_proof_and_curve::<
        Bn254,
        UnivariateDeKARTv2<Bn254>,
    >();
    assert_correctness_and_serialization_for_range_proof_and_curve::<
        Bls12_381,
        UnivariateDeKARTv2<Bls12_381>,
    >();

    assert_correctness_and_serialization_for_range_proof_and_curve::<
        Bn254,
        DekartMultivariate<Bn254>,
    >();
    assert_correctness_and_serialization_for_range_proof_and_curve::<
        Bls12_381,
        DekartMultivariate<Bls12_381>,
    >();
}
