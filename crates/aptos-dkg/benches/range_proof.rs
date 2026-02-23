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
    utils::test_utils::{self},
};
use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::pairing::Pairing;
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
};
use rand::{rngs::StdRng, SeedableRng};

/// WARNING: Do not change this, since our range proof benchmark instructions in
/// `crates/aptos-crypto/README.md` rely on it.
const BROKEN_DEKART_RS_SCHEME_NAME: &str = "dekart-rs-broken";
const DEKART_RS_SCHEME_NAME: &str = "dekart-rs";
const DEKART_MULTIVARIATE_SCHEME_NAME: &str = "dekart-multivar";
const BN254: &str = "bn254";
const BLS12_381: &str = "bls12-381";

/// WARNING: These are the relevant batch sizes we want benchmarked to compare against Bulletproofs
//const BATCH_SIZES: [usize; 11] = [1, 3, 7, 15, 31, 63, 127, 255, 511, 1023, 2047];
const BATCH_SIZES: [usize; 3] = [1023, 16383, 131071]; //100000, 1000000];

/// WARNING: These are the relevant bit widths we want benchmarked to compare against Bulletproofs
const BIT_WIDTHS: [u8; 4] = [8, 16, 32, 64];

fn bench_groups(c: &mut Criterion) {
    bench_range_proof::<Bn254, UnivariateDeKART<Bn254>>(c, BROKEN_DEKART_RS_SCHEME_NAME, BN254);
    bench_range_proof::<Bls12_381, UnivariateDeKART<Bls12_381>>(
        c,
        BROKEN_DEKART_RS_SCHEME_NAME,
        BLS12_381,
    );

    bench_range_proof::<Bn254, UnivariateDeKARTv2<Bn254>>(c, DEKART_RS_SCHEME_NAME, BN254);
    bench_range_proof::<Bls12_381, UnivariateDeKARTv2<Bls12_381>>(
        c,
        DEKART_RS_SCHEME_NAME,
        BLS12_381,
    );

    bench_range_proof::<Bn254, DekartMultivariate<Bn254>>(c, DEKART_MULTIVARIATE_SCHEME_NAME, BN254);
    bench_range_proof::<Bls12_381, DekartMultivariate<Bls12_381>>(
        c,
        DEKART_MULTIVARIATE_SCHEME_NAME,
        BLS12_381,
    );
}

/// Generic benchmark function over any pairing curve and range proof
fn bench_range_proof<E: Pairing, B: BatchedRangeProof<E>>(
    c: &mut Criterion,
    scheme_name: &str,
    curve_name: &str,
) {
    let mut group = c.benchmark_group(format!("{}/{}", scheme_name, curve_name));

    let l = std::env::var("L").ok().and_then(|s| s.parse::<u8>().ok());
    let n = std::env::var("N")
        .ok()
        .and_then(|s| s.parse::<usize>().ok());

    match (l, n) {
        (Some(ell), Some(n)) => {
            bench_prove::<E, B>(&mut group, ell, n);
            bench_verify::<E, B>(&mut group, ell, n);
        },
        (_, _) => {
            for n in BATCH_SIZES {
                for ell in BIT_WIDTHS {
                    bench_prove::<E, B>(&mut group, ell, n);
                    bench_verify::<E, B>(&mut group, ell, n);
                }
            }
        },
    }
}

fn bench_verify<E: Pairing, B: BatchedRangeProof<E>>(
    group: &mut BenchmarkGroup<WallTime>,
    ell: u8,
    n: usize,
) {
    group.bench_function(
        BenchmarkId::new("verify", format!("ell={ell}/n={n}")),
        |b| {
            b.iter_with_setup(
                || {
                    let mut rng = StdRng::seed_from_u64(42); // TODO: hmm not ideal to put this here
                    let group_generators = GroupGenerators::default();
                    let (pk, vk) = B::setup(n, ell, group_generators, &mut rng);
                    let (values, comm, r) =
                        test_utils::range_proof_random_instance::<_, B, _>(&pk, n, ell, &mut rng);
                    let proof = B::prove(&pk, &values, ell, &comm, &r, &mut rng);
                    (vk, n, ell, comm, proof, rng)
                },
                |(vk, n, ell, comm, proof, mut rng)| {
                    proof.verify(&vk, n, ell, &comm, &mut rng).unwrap();
                },
            )
        },
    );
}

fn bench_prove<E: Pairing, B: BatchedRangeProof<E>>(
    group: &mut BenchmarkGroup<WallTime>,
    ell: u8,
    n: usize,
) {
    group.bench_function(
        BenchmarkId::new("prove", format!("ell={ell}/n={n}")),
        move |b| {
            b.iter_with_setup(
                || {
                    let mut rng = StdRng::seed_from_u64(42);
                    let group_generators = GroupGenerators::default();
                    let (pk, _) = B::setup(n, ell, group_generators, &mut rng);
                    let (values, comm, r) =
                        test_utils::range_proof_random_instance::<_, B, _>(&pk, n, ell, &mut rng);
                    (pk, values, comm, r, rng)
                },
                |(pk, values, comm, r, mut rng)| {
                    let _proof = B::prove(&pk, &values, ell, &comm, &r, &mut rng);
                },
            )
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_groups
);
criterion_main!(benches);
