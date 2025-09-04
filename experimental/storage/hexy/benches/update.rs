// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::HashValue;
use velor_experimental_hexy::{
    in_mem::{base::HexyBase, overlay::HexyOverlay},
    LeafIdx,
};
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BatchSize, BenchmarkGroup, Criterion,
};
use rand::Rng;
use std::sync::Arc;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

const M: usize = 1024 * 1024;
const K: usize = 1024;

fn gen_hexy_updates(batch_size_k: usize, set_size_m: usize) -> Vec<(LeafIdx, HashValue)> {
    (0..batch_size_k * K)
        .map(|_| {
            (
                rand::thread_rng().gen_range(0, (set_size_m * M) as LeafIdx),
                HashValue::random(),
            )
        })
        .collect()
}

fn hexy_update(
    group: &mut BenchmarkGroup<WallTime>,
    batch_size_k: usize,
    set_size_m: usize,
    pipeline_depth: usize,
) {
    println!("Allocating base: {set_size_m}M items");
    let base = Arc::new(HexyBase::allocate((set_size_m * M) as u32));
    let root_overlay = HexyOverlay::new_empty(&base);
    let mut base_overlay = root_overlay.clone();
    println!("Prepare pipeline of depth {pipeline_depth}");
    for _ in 0..pipeline_depth {
        let updates = gen_hexy_updates(batch_size_k, set_size_m);
        base_overlay = base_overlay
            .view(&base, &root_overlay)
            .new_overlay(updates)
            .unwrap();
    }
    let updates = gen_hexy_updates(batch_size_k, set_size_m);

    group.throughput(criterion::Throughput::Elements(batch_size_k as u64 * 1024));
    let name = format!(
        "hexy_update_leaves_{}m_batch_{}k_pipeline_depth_{}",
        set_size_m, batch_size_k, pipeline_depth
    );
    group.bench_function(&name, |b| {
        b.iter_batched(
            || updates.clone(),
            |updates| {
                base_overlay
                    .view(&base, &root_overlay)
                    .new_overlay(updates)
                    .unwrap()
            },
            BatchSize::SmallInput,
        )
    });
}

fn hexy_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("hexy_updates".to_string());

    for set_size_m in [32, 64, 128] {
        for batch_size_k in [1, 10] {
            for pipeline_depth in [0, 2, 8] {
                hexy_update(&mut group, batch_size_k, set_size_m, pipeline_depth);
            }
        }
    }
}

criterion_group!(
    name = update;
    config = Criterion::default();
    targets = hexy_updates
);

criterion_main!(update);
