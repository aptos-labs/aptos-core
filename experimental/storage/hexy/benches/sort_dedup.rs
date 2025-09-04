// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_experimental_hexy::utils::sort_dedup;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use itertools::Itertools;
use std::collections::BTreeMap;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_dedup");

    const SET_SIZE: usize = 100000;

    let data = std::iter::repeat_with(rand::random::<(u32, u64)>)
        .take(SET_SIZE)
        .collect_vec();
    group.throughput(criterion::Throughput::Elements(SET_SIZE as u64));

    group.bench_function("my_sort_dedup", |b| {
        b.iter_batched(|| data.clone(), sort_dedup, BatchSize::SmallInput);
    });

    group.bench_function("btree_sort_dedup", |b| {
        b.iter_batched(
            || data.clone(),
            |data| {
                data.into_iter()
                    .collect::<BTreeMap<_, _>>()
                    .into_iter()
                    .collect_vec()
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench
);

criterion_main!(benches);
