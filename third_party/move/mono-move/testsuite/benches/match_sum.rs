// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[path = "support/mod.rs"]
mod support;

use criterion::{black_box, measurement::Measurement, BenchmarkId, Criterion};
use mono_move_testsuite::{
    programs::{
        match_sum::{move_bytecode_match_sum, native_match_sum, SOURCE},
        testing,
    },
    with_loaded_mono_function, SourceKind,
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

const N: u64 = 1_000_000;

fn mono<M: Measurement>(c: &mut Criterion<M>, metric: &str) {
    let mut group = c.benchmark_group("match_sum");
    let addr = AccountAddress::from_hex_literal("0x1").unwrap();
    with_loaded_mono_function(
        SOURCE,
        SourceKind::Move,
        addr,
        IdentStr::new("match_sum").unwrap(),
        IdentStr::new("match_sum").unwrap(),
        |runner| {
            group.bench_function(BenchmarkId::new("mono", metric), |b| {
                b.iter(|| black_box(runner.call_words(&[N]).unwrap()));
            });
        },
    )
    .unwrap();
    group.finish();
}

/// Reference points: native Rust and the production Move VM.
fn controls(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_sum");
    group.bench_function("native", |b| {
        b.iter(|| black_box(native_match_sum(N)));
    });
    group.finish();

    // The Move VM is slower, so it gets a smaller sample.
    let mut group = c.benchmark_group("match_sum");
    group.sample_size(10);
    let module = move_bytecode_match_sum();
    testing::with_loaded_move_function(&module, "match_sum", |env| {
        group.bench_function("move_vm", |b| {
            b.iter(|| {
                let result = env.run(vec![testing::arg_u64(N)]);
                black_box(testing::return_u64(&result))
            });
        });
    });
    group.finish();
}

support::bench_main!(mono, controls);
