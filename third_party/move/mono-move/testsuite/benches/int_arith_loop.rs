// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Microbenchmark isolating arithmetic dispatch cost: `u64_loop` and
//! `i64_loop` run the same loop, so the u64 (specialized) vs i64
//! (unspecialized) delta is the per-op dispatch difference.

#[path = "support/mod.rs"]
mod support;

use criterion::{black_box, measurement::Measurement, BenchmarkId, Criterion};
use mono_move_testsuite::{
    programs::{
        int_arith_loop::{move_bytecode_int_arith_loop, native_i64_loop, native_u64_loop, SOURCE},
        testing,
    },
    with_loaded_mono_function, SourceKind,
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

const ITERS: u64 = 1_000;

fn mono<M: Measurement>(c: &mut Criterion<M>, metric: &str) {
    let mut group = c.benchmark_group("int_arith_loop");
    let addr = AccountAddress::from_hex_literal("0x1").unwrap();
    with_loaded_mono_function(
        SOURCE,
        SourceKind::Move,
        addr,
        IdentStr::new("int_arith_loop").unwrap(),
        IdentStr::new("u64_loop").unwrap(),
        |runner| {
            group.bench_function(BenchmarkId::new("mono_u64", metric), |b| {
                b.iter(|| black_box(runner.call_words(&[ITERS]).unwrap()));
            });
        },
    )
    .unwrap();
    with_loaded_mono_function(
        SOURCE,
        SourceKind::Move,
        addr,
        IdentStr::new("int_arith_loop").unwrap(),
        IdentStr::new("i64_loop").unwrap(),
        |runner| {
            group.bench_function(BenchmarkId::new("mono_i64", metric), |b| {
                // Same 8 bytes; reinterpret as i64.
                b.iter(|| black_box(runner.call_words(&[ITERS]).unwrap() as i64));
            });
        },
    )
    .unwrap();
    group.finish();
}

/// Reference points: native Rust and the production Move VM.
fn controls(c: &mut Criterion) {
    let mut group = c.benchmark_group("int_arith_loop");
    group.bench_function("native_u64", |b| {
        b.iter(|| black_box(native_u64_loop(ITERS)));
    });
    group.bench_function("native_i64", |b| {
        b.iter(|| black_box(native_i64_loop(ITERS)));
    });
    group.finish();

    // The Move VM is slower, so it gets a smaller sample.
    let mut group = c.benchmark_group("int_arith_loop");
    group.sample_size(10);
    let module = move_bytecode_int_arith_loop();
    testing::with_loaded_move_function(&module, "u64_loop", |env| {
        group.bench_function("move_vm_u64", |b| {
            b.iter(|| {
                let result = env.run(vec![testing::arg_u64(ITERS)]);
                black_box(testing::return_u64(&result))
            });
        });
    });
    testing::with_loaded_move_function(&module, "i64_loop", |env| {
        group.bench_function("move_vm_i64", |b| {
            b.iter(|| {
                let result = env.run(vec![testing::arg_u64(ITERS)]);
                black_box(testing::return_i64(&result))
            });
        });
    });
    group.finish();
}

support::bench_main!(mono, controls);
