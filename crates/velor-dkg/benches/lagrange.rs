// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_dkg::algebra::{
    evaluation_domain::BatchEvaluationDomain, lagrange::lagrange_coefficients,
};
use blstrs::Scalar;
use criterion::{
    criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup, BenchmarkId,
    Criterion, Throughput,
};
use ff::Field;
use more_asserts::{assert_ge, assert_le};
use rand::{seq::IteratorRandom, thread_rng};

pub fn lagrange_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("lagrange");

    lagrange_tcz20(333, 1_000, &mut group);
    lagrange_tcz20(666, 1_000, &mut group);
    lagrange_tcz20(3333, 10_000, &mut group);
    lagrange_tcz20(6666, 10_000, &mut group);

    group.finish();
}

#[allow(non_snake_case)]
pub fn lagrange_tcz20<M: Measurement>(thresh: usize, n: usize, g: &mut BenchmarkGroup<M>) {
    assert_ge!(thresh, 1);
    assert_le!(thresh, n);
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(n as u64));

    g.bench_function(
        BenchmarkId::new(format!("tcz20-thresh={thresh}"), n),
        move |b| {
            b.iter_with_setup(
                || {
                    let players: Vec<usize> = (0..n)
                        .choose_multiple(&mut rng, thresh)
                        .into_iter()
                        .collect::<Vec<usize>>();

                    let batch_dom = BatchEvaluationDomain::new(n);

                    (players, batch_dom)
                },
                |(players, batch_dom)| {
                    lagrange_coefficients(&batch_dom, players.as_slice(), &Scalar::ZERO);
                },
            )
        },
    );
}

criterion_group!(
    name = benches;
    //config = Criterion::default().sample_size(10);
    config = Criterion::default();
    targets = lagrange_group);
criterion_main!(benches);
