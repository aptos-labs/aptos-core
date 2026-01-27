// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_borrow)]

use aptos_crypto::arkworks;
use aptos_dkg::{
    algebra::evaluation_domain::BatchEvaluationDomain,
    pvss::{test_utils::BENCHMARK_CONFIGS, LowDegreeTest},
};
use ark_poly::EvaluationDomain;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::thread_rng;

pub fn all_groups(c: &mut Criterion) {
    arkworks_ldt_group(c);
    blstrs_ldt_group(c);
}

pub fn blstrs_ldt_group(c: &mut Criterion) {
    let mut rng = thread_rng();
    let mut group = c.benchmark_group("blstrs_ldt");

    for &(t, n) in BENCHMARK_CONFIGS {
        group.bench_function(format!("dual_code_word/t{}/n{}", t, n), |b| {
            b.iter_with_setup(
                || {
                    let batch_dom = BatchEvaluationDomain::new(n);
                    (n, t, batch_dom)
                },
                |(n, t, batch_dom)| {
                    let ldt = LowDegreeTest::random(&mut rng, t, n, true, &batch_dom);
                    ldt.dual_code_word();
                },
            )
        });
    }
}

pub fn arkworks_ldt_group(c: &mut Criterion) {
    let mut rng = thread_rng();
    let mut group = c.benchmark_group("arkworks_ldt");

    for &(t, n) in BENCHMARK_CONFIGS {
        group.bench_function(format!("dual_code_word/t{}/n{}", t, n), |b| {
            b.iter_with_setup(
                || {
                    let batch_dom =
                        ark_poly::Radix2EvaluationDomain::<ark_bn254::Fr>::new(n).unwrap();
                    (n, t, batch_dom)
                },
                |(n, t, batch_dom)| {
                    let ldt =
                        arkworks::scrape::LowDegreeTest::random(&mut rng, t, n, true, &batch_dom);
                    ldt.dual_code_word();
                },
            )
        });
    }
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = all_groups);
criterion_main!(benches);
