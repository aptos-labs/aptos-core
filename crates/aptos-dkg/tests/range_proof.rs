// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::range_proof::{batch_prove, batch_verify, commit, setup, DST};
use blstrs::Scalar;
use rand::{thread_rng, RngCore};

#[cfg(test)]
fn run_range_proof_completeness(n: usize, ell: usize) {
    let mut rng = thread_rng();
    let pp = setup(ell, n);
    println!("setup finished for n={}, ell={}, prove starting", n, ell);

    let zz: Vec<Scalar> = (0..n)
        .map(|_| {
            let val = rng.next_u64() >> (64 - ell);
            Scalar::from(val)
        })
        .collect();

    let (cc, r) = commit(&pp, &zz, &mut rng);

    let mut fs_t = merlin::Transcript::new(DST);
    let proof = batch_prove(&mut rng, &pp, &zz, &cc, &r, &mut fs_t);
    println!("prove finished, vrfy1 starting (n={}, ell={})", n, ell);

    let mut fs_t = merlin::Transcript::new(DST);
    batch_verify(&pp, &cc, &proof, &mut fs_t).unwrap();

    println!("vrfy finished, vrfy2 starting (n={}, ell={})", n, ell);
    let mut invalid_proof = proof.clone();
    invalid_proof.maul();
    let mut fs_t = merlin::Transcript::new(DST);
    assert!(batch_verify(&pp, &cc, &invalid_proof, &mut fs_t).is_err())
}

#[cfg(test)]
fn run_serialize_range_proof(n: usize, ell: usize) {
    let mut rng = thread_rng();
    let pp = setup(ell, n);

    println!("setup finished for n={}, ell={}, prove starting", n, ell);

    let zz: Vec<Scalar> = (0..n)
        .map(|_| {
            let val = rng.next_u64() >> (64 - ell);
            Scalar::from(val)
        })
        .collect();

    let (cc, r) = commit(&pp, &zz, &mut rng);
    let mut fs_t = merlin::Transcript::new(DST);
    let proof = batch_prove(&mut rng, &pp, &zz, &cc, &r, &mut fs_t);

    // === Serialize to memory ===
    let encoded = bcs::to_bytes(&proof).expect("Serialization failed");
    println!(
        "Serialized proof size (n={}, ell={}): {} bytes, expected for blstrs: {} bytes",
        n,
        ell,
        encoded.len(),
        2 * 8 + 48 + (48 + 96) * ell // Can get rid of the 2 * 8 by turning the Vecs in `proof` into tuples
    );

    // === Round-trip deserialization ===
    let decoded = bcs::from_bytes(&encoded).expect("Deserialization failed");

    // Verify still succeeds
    let mut fs_t = merlin::Transcript::new(DST);
    batch_verify(&pp, &cc, &decoded, &mut fs_t).unwrap();

    // Make invalid
    let mut invalid_proof = decoded.clone();
    invalid_proof.maul();
    let mut fs_t = merlin::Transcript::new(DST);
    assert!(batch_verify(&pp, &cc, &invalid_proof, &mut fs_t).is_err());

    println!(
        "Serialization round-trip test passed for n={}, ell={}",
        n, ell
    );
}

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
    (255, 16),
    (255, 32),
    (512, 32),
    (1024, 32),
    (2047, 32),
];

#[test]
fn range_proof_completeness_multi() {
    for &(n, ell) in TEST_CASES {
        run_range_proof_completeness(n, ell);
    }
}

#[test]
fn serialize_range_proof_multi() {
    for &(n, ell) in TEST_CASES {
        run_serialize_range_proof(n, ell);
    }
}
