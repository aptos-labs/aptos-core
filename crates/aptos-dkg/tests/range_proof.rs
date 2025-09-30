// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use aptos_dkg::range_proofs::traits::BatchedRangeProof;
use aptos_dkg::{
    range_proofs::dekart_univariate::{Proof, DST},
    utils::test_utils,
};
use ark_ec::pairing::Pairing;
use ark_std::rand::thread_rng;

#[cfg(test)]
fn run_range_proof_completeness<E: Pairing, B: BatchedRangeProof<E>>(n: usize, ell: usize) {
    let mut rng = thread_rng();
    let (pk, vk, values, comm, r) =
        test_utils::range_proof_random_instance::<E, B, _>(n, ell, &mut rng);
    println!("setup finished for n={}, ell={}, prove starting", n, ell);

    let mut fs_t = merlin::Transcript::new(DST);
    let proof = B::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
    println!("prove finished, vrfy1 starting (n={}, ell={})", n, ell);

    let mut fs_t = merlin::Transcript::new(DST);
    proof.verify(&vk, n, ell, &comm, &mut fs_t).unwrap();

    println!("vrfy finished, vrfy2 starting (n={}, ell={})", n, ell);
    let mut invalid_proof = proof.clone();
    invalid_proof.maul();
    let mut fs_t = merlin::Transcript::new(DST);
    assert!(invalid_proof.verify(&vk, n, ell, &comm, &mut fs_t).is_err())
}

#[cfg(test)]
fn run_serialize_range_proof<E: Pairing, B: BatchedRangeProof<E>>(n: usize, ell: usize) {
    let mut rng = thread_rng();
    let (pk, vk, values, comm, r) =
        test_utils::range_proof_random_instance::<E, B, _>(n, ell, &mut rng);
    println!("setup finished for n={}, ell={}, prove starting", n, ell);

    let mut fs_t = merlin::Transcript::new(DST);
    let proof = B::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);

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
    let mut fs_t = merlin::Transcript::new(DST);
    decoded.verify(&vk, n, ell, &comm, &mut fs_t).unwrap();

    // Make invalid
    let mut invalid_proof = decoded.clone();
    invalid_proof.maul();
    let mut fs_t = merlin::Transcript::new(DST);
    assert!(invalid_proof.verify(&vk, n, ell, &comm, &mut fs_t).is_err());

    println!(
        "Serialization round-trip test passed for n={}, ell={}",
        n, ell
    );
}

#[cfg(test)]
const TEST_CASES: &[(usize, usize)] = &[
    // (n, \ell)
    (3, 16),
    (7, 16),
    (4, 16),
    (8, 16),
    (16, 3),
    (16, 4),
    (16, 7),
    (16, 8),
    (16, 16),
    (255, 16),
    (255, 32),
    (512, 32),
    (1024, 32),
    (2047, 32),
];

#[cfg(test)]
macro_rules! for_each_curve {
    ($f:ident, $n:expr, $ell:expr) => {
        use ark_bls12_381::Bls12_381;
        use ark_bn254::Bn254;

        $f::<Bn254, Proof<Bn254>>($n, $ell);
        $f::<Bls12_381, Proof<Bls12_381>>($n, $ell);
    };
}

#[test]
fn range_proof_completeness_multi() {
    for &(n, ell) in TEST_CASES {
        for_each_curve!(run_range_proof_completeness, n, ell);
    }
}

#[test]
fn serialize_range_proof_multi() {
    for &(n, ell) in TEST_CASES {
        for_each_curve!(run_serialize_range_proof, n, ell);
    }
}
