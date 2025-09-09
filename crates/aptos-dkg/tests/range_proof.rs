// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::range_proof::{batch_prove, batch_verify, commit, setup};
use blstrs::Scalar;
use rand::thread_rng;
use rand_core::RngCore;

#[test]
fn range_proof_completeness() {
    let mut rng = thread_rng();

    let ell = std::env::var("L")
        .unwrap_or(std::env::var("ELL").unwrap_or_default())
        .parse::<usize>()
        .unwrap_or(16);

    let n = std::env::var("N")
        .unwrap_or_default()
        .parse::<usize>()
        .unwrap_or(127);

    let pp = setup(ell, n);
    println!("setup finished, prove starting");
    let zz: Vec<Scalar> = (0..n)
        .map(|_| {
            let val = rng.next_u64() >> (64 - ell);
            Scalar::from(val)
        })
        .collect();
    let (cc, r) = commit(&pp, &zz, &mut rng);
    let proof = batch_prove(&mut rng, &pp, &zz, &cc, &r);
    println!(
        "proof size for \\ell = {} and n = {} is {} bytes",
        ell,
        n,
        48 + (48 + 96) * ell
    );
    println!("prove finished, vrfy1 starting");
    batch_verify(&pp, &cc, &proof).unwrap();

    println!("vrfy finished, vrfy2 starting");
    let mut invalid_proof = proof.clone();
    invalid_proof.maul();
    assert!(batch_verify(&pp, &cc, &invalid_proof).is_err())
}
