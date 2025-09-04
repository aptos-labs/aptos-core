// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use velor_crypto::{poseidon_bn254, test_utils::random_bytes};
use ark_ff::PrimeField;
use criterion::{measurement::Measurement, BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use poseidon_ark::Poseidon;
use rand::thread_rng;

/// Runs all the benchmarks.
fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("zk");

    //let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);
    //group.plot_config(plot_config);
    group.sample_size(50);

    for n in 1..=16 {
        poseidon_bn254_ark(&mut group, n);
        poseidon_bn254_neptune(&mut group, n);
    }

    group.finish();
}

fn poseidon_bn254_neptune<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(n as u64));

    g.bench_function(BenchmarkId::new("poseidon-bn254-neptune", n), move |b| {
        b.iter_with_setup(
            || {
                (0..n)
                    .map(|_| {
                        let bytes = random_bytes(&mut rng, n);
                        ark_bn254::Fr::from_le_bytes_mod_order(bytes.as_slice())
                    })
                    .collect::<Vec<ark_bn254::Fr>>()
            },
            |scalars| {
                assert_eq!(scalars.len(), n);

                poseidon_bn254::hash_scalars(scalars).unwrap()
            },
        )
    });
}

fn poseidon_bn254_ark<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(n as u64));

    let hash = Poseidon::new();

    g.bench_function(BenchmarkId::new("poseidon-bn254-ark", n), move |b| {
        b.iter_with_setup(
            || {
                (0..n)
                    .map(|_| {
                        let bytes = random_bytes(&mut rng, n);
                        ark_bn254::Fr::from_le_bytes_mod_order(bytes.as_slice())
                    })
                    .collect::<Vec<ark_bn254::Fr>>()
            },
            |scalars| {
                assert_eq!(scalars.len(), n);

                hash.hash(scalars).unwrap()
            },
        )
    });
}

criterion_group!(
    name = zk_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(zk_benches);
