// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use aptos_types::shared_bytes::SharedBytes;
use bytes::Bytes;
use criterion::{measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn for_sizes<F>(c: &mut Criterion, name: &'static str, bench: F)
where
    F: Fn(&mut BenchmarkGroup<WallTime>, usize),
{
    let mut group = c.benchmark_group(name);
    for n_bytes in [16, 128, 512, 1024, 2048, 4096] {
        bench(&mut group, n_bytes);
    }
    group.finish();
}

fn benches(c: &mut Criterion) {
    for_sizes(c, "copy_construct", copy_construct);
    for_sizes(c, "clone", clone);
    for_sizes(c, "bcs_serialize", bcs_serialize);
    for_sizes(c, "bcs_deserialize", bcs_deserialize);
}

fn buf_of_size(n_bytes: usize) -> Vec<u8> {
    let mut vec = Vec::with_capacity(n_bytes);
    vec.resize(n_bytes, 3u8);
    vec
}

fn id(func: &'static str, n_bytes: usize) -> BenchmarkId {
    BenchmarkId::new(func, n_bytes)
}

fn copy_construct(g: &mut BenchmarkGroup<WallTime>, n_bytes: usize) {
    let source_buf: Vec<u8> = buf_of_size(n_bytes);
    let s = &source_buf[..];

    g.bench_function(id("vec", n_bytes), |b| b.iter(|| s.to_vec()));
    g.bench_function(id("box_vec", n_bytes), |b| b.iter(|| Box::new(s.to_vec())));
    g.bench_function(id("bytes", n_bytes), |b| {
        b.iter(|| Bytes::copy_from_slice(s))
    });
    g.bench_function(id("shared_bytes", n_bytes), |b| {
        b.iter(|| SharedBytes::copy(s))
    });
}

fn clone(g: &mut BenchmarkGroup<WallTime>, n_bytes: usize) {
    let source_buf: Vec<u8> = buf_of_size(n_bytes);
    let vec = source_buf.clone();
    let box_vec = Box::new(source_buf.clone());
    let bytes = Bytes::copy_from_slice(&source_buf[..]);
    let shared_bytes = SharedBytes::copy(&source_buf[..]);

    g.bench_function(id("vec", n_bytes), |b| b.iter(|| vec.clone()));
    g.bench_function(id("box_vec", n_bytes), |b| b.iter(|| box_vec.clone()));
    g.bench_function(id("bytes", n_bytes), |b| b.iter(|| bytes.clone()));
    g.bench_function(id("shared_bytes", n_bytes), |b| {
        b.iter(|| shared_bytes.clone())
    });
}

#[derive(Deserialize, Serialize)]
struct VecWrapper(#[serde(with = "serde_bytes")] Vec<u8>);

fn bcs_serialize(g: &mut BenchmarkGroup<WallTime>, n_bytes: usize) {
    let source_buf: Vec<u8> = buf_of_size(n_bytes);
    let vec = source_buf.clone();
    let vec_wrapper = VecWrapper(source_buf.clone());
    let bytes = Bytes::copy_from_slice(&source_buf[..]);
    let shared_bytes = SharedBytes::copy(&source_buf[..]);

    g.bench_function(id("vec", n_bytes), |b| {
        b.iter(|| bcs::to_bytes(&vec).unwrap())
    });
    g.bench_function(id("vec_serde_bytes", n_bytes), |b| {
        b.iter(|| bcs::to_bytes(&vec_wrapper).unwrap())
    });
    g.bench_function(id("bytes", n_bytes), |b| {
        b.iter(|| bcs::to_bytes(bytes.as_ref()).unwrap())
    });
    g.bench_function(id("shared_bytes", n_bytes), |b| {
        b.iter(|| bcs::to_bytes(&shared_bytes).unwrap())
    });
}

fn bcs_deserialize(g: &mut BenchmarkGroup<WallTime>, n_bytes: usize) {
    let source_buf: Vec<u8> = buf_of_size(n_bytes);
    let source_buf: Vec<u8> = bcs::to_bytes(&source_buf).unwrap();
    let s = source_buf.as_slice();

    g.bench_function(id("vec", n_bytes), |b| {
        b.iter(|| {
            let vec: Vec<u8> = bcs::from_bytes(s).unwrap();
            vec
        })
    });
    g.bench_function(id("vec_serde_bytes", n_bytes), |b| {
        b.iter(|| {
            let vec_wrapper: VecWrapper = bcs::from_bytes(s).unwrap();
            vec_wrapper
        })
    });
    g.bench_function(id("shared_bytes", n_bytes), |b| {
        b.iter(|| {
            let shared_bytes: SharedBytes = bcs::from_bytes(s).unwrap();
            shared_bytes
        })
    });
}

criterion_group!(
    name = shared_bytes;
    config = Criterion::default();
    targets = benches
);
criterion_main!(shared_bytes);
