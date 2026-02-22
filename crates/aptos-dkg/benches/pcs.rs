// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_dkg::pcs::{
    traits::{random_point, random_poly, PolynomialCommitmentScheme},
    zeromorph::Zeromorph,
};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use rand::{rngs::StdRng, SeedableRng};
use std::hint::black_box;

/// Generic benchmark harness for any commitment scheme implementing `CommitmentScheme`.
///
/// Benchmarks the three core operations:
/// - Commit
/// - Open (prove an evaluation)
/// - Verify (verify the opening proof)
fn benchmark_commitment_scheme<PCS: PolynomialCommitmentScheme>(c: &mut Criterion) {
    // Create a benchmark group labeled with the scheme name
    let mut group = c.benchmark_group(format!(
        "commitment_scheme/{}",
        String::from_utf8_lossy(PCS::scheme_name())
    ));

    // Polynomial sizes to benchmark (powers of two); corresponds to rounding up [1_000, 10_000, 100_000, 1_000_000]
    let sizes: [u32; 4] = [1 << 10, 1 << 14, 1 << 17, 1 << 20];

    for len in sizes {
        // Use a fixed seed so all runs are deterministic and comparable
        let mut rng = StdRng::seed_from_u64(0xDEAD_BEEF);

        // Number of variables needed to represent a multilinear polynomial of length `len`
        let num_vars = len.next_power_of_two().ilog2();

        // --------------------------------------------------
        // Setup phase (trusted / structured reference string)
        // --------------------------------------------------
        // This is intentionally done once per size and excluded from benchmarks; becomes quite slow for large `num_vars`
        println!("Computing setup...");
        let (ck, vk) = PCS::setup(vec![1; num_vars as usize], &mut rng);
        println!("Finished setup");

        // ------------------------------------------
        // Benchmark Commit
        // ------------------------------------------
        // Measures the cost of committing to a polynomial
        group.bench_with_input(BenchmarkId::new("commit", len), &len, |b, &_len| {
            b.iter_batched(
                || random_poly::<PCS, _>(&mut rng, len, Some(32)),
                |poly| {
                    PCS::commit(&ck, poly, None);
                },
                BatchSize::LargeInput,
            );
        });

        // ------------------------------------------
        // Benchmark Open
        // ------------------------------------------
        // Measures the cost of generating an evaluation proof
        group.bench_with_input(BenchmarkId::new("open", len), &len, |b, &_len| {
            b.iter_batched(
                || {
                    let poly = random_poly::<PCS, _>(&mut rng, len, Some(32));
                    let challenge = random_point::<PCS, _>(&mut rng, num_vars);
                    let mut rng = rand::thread_rng();
                    let r = PCS::random_witness(&mut rng);
                    let trs = merlin::Transcript::new(b"pcs-bench");
                    (poly, challenge, Some(r), rng, trs)
                },
                |(poly, challenge, r, mut rng, mut trs)| {
                    PCS::open(&ck, poly, challenge, r, &mut rng, &mut trs);
                },
                BatchSize::LargeInput,
            );
        });

        // ------------------------------------------
        // Benchmark Verify
        // ------------------------------------------
        group.bench_with_input(BenchmarkId::new("verify", len), &len, |b, &_len| {
            b.iter_batched(
                || {
                    let poly = random_poly::<PCS, _>(&mut rng, len, Some(32));
                    let challenge = random_point::<PCS, _>(&mut rng, num_vars);
                    let val = PCS::evaluate_point(&poly, &challenge);
                    let com = PCS::commit(&ck, poly.clone(), None);
                    let mut rng = rand::thread_rng();
                    let r = PCS::random_witness(&mut rng);
                    let mut trs = merlin::Transcript::new(b"pcs-bench");
                    let proof = PCS::open(
                        &ck,
                        poly.clone(),
                        challenge.clone(),
                        Some(r),
                        &mut rng,
                        &mut trs,
                    );
                    (challenge, val, com, proof, trs)
                },
                |(challenge, val, com, proof, mut trs)| {
                    let _ =
                        PCS::verify(black_box(&vk), com, challenge, val, proof, &mut trs, false);
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

/// Benchmark entry point for the Zeromorph commitment scheme instantiated
/// over the BLS12-381 pairing-friendly curve.
fn bench_zeromorph(c: &mut Criterion) {
    type E = ark_bls12_381::Bls12_381;

    benchmark_commitment_scheme::<Zeromorph<E>>(c);
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = bench_zeromorph);

criterion_main!(benches);
