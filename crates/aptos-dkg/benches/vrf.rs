// Copyright © Aptos Foundation

use criterion::{
    criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup, BenchmarkId,
    Criterion, Throughput,
};
use rand::thread_rng;
use std::ops::Mul;

use aptos_dkg::constants::BEST_CASE_THRESHOLD;
use aptos_dkg::utils::random::{random_g1_points, random_g2_points, random_scalars};
use aptos_dkg::utils::{g1_multi_exp, g2_multi_exp, multi_pairing};

pub fn vrf_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("vrf");

    for t in [10, BEST_CASE_THRESHOLD] {
        unoptimized_threshold_vuf_share_verification(t, &mut group);
    }

    group.finish();
}

fn unoptimized_threshold_vuf_share_verification<M: Measurement>(
    t: usize,
    g: &mut BenchmarkGroup<M>,
) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1u64));

    g.bench_function(
        BenchmarkId::new("unoptimized_threshold_vuf_share_verification", t),
        move |b| {
            b.iter_with_setup(
                || {
                    let g1 = random_g1_points(t + 3, &mut rng);
                    let g2 = random_g2_points(t + 3, &mut rng);
                    let a = random_scalars(t + 3, &mut rng);
                    let b = random_scalars(3, &mut rng);
                    let c = random_scalars(3, &mut rng);

                    (g1, g2, a, b, c)
                },
                |(g1, g2, a, b, c)| {
                    // 2 G_1 exps
                    let _ = g1[0].mul(a[0]);
                    let _ = g1[1].mul(a[1]);
                    // size-3 multipairing
                    multi_pairing(g1.iter().take(3), g2.iter().take(3));
                    // size-4 multipairing
                    multi_pairing(g1.iter().take(4), g2.iter().take(4));
                    // 3 size-2 G_1 multiexps
                    g1_multi_exp(&g1[0..2], &a[0..2]);
                    g1_multi_exp(&g1[0..2], &b[0..2]);
                    g1_multi_exp(&g1[0..2], &c[0..2]);
                    // 1 size-t G_1 multiexp
                    g1_multi_exp(&g1[0..t], &a[0..t]);
                    // 1 size-t G_2 multiexp
                    g2_multi_exp(&g2[0..t], &a[0..t]);
                },
            )
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    //config = Criterion::default();
    targets = vrf_group);
criterion_main!(benches);
