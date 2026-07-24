// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `vector::move_range` microbenchmark, used to compare lowering the call as a
//! native versus a dedicated micro-op: run the same source on both the
//! native-function and micro-op branches and compare the `mono_*` numbers.
//! `small` moves a 2-element chunk (per-call dispatch dominates); `large` moves
//! a 60-element chunk (the element memcpy dominates, which is identical for both
//! lowerings). No `move_vm` flavor — `move_range` is a native there and the
//! bench's legacy harness registers no natives; the comparison is between the
//! two mono-move lowerings, with the native Rust baseline as the floor.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const ITERS: u64 = 5_000;

fn bench_move_range(c: &mut Criterion) {
    use mono_move_testsuite::{
        programs::move_range_bench::{native_shuffle_large, native_shuffle_small, SOURCE},
        with_loaded_mono_function, SourceKind,
    };
    use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

    let addr = AccountAddress::from_hex_literal("0x1").unwrap();

    let mut group = c.benchmark_group("move_range");
    group
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(3));

    group.bench_function("native_small", |b| {
        b.iter(|| black_box(native_shuffle_small(ITERS)));
    });
    group.bench_function("native_large", |b| {
        b.iter(|| black_box(native_shuffle_large(ITERS)));
    });

    with_loaded_mono_function(
        SOURCE,
        SourceKind::Move,
        addr,
        IdentStr::new("move_range_bench").unwrap(),
        IdentStr::new("shuffle_small").unwrap(),
        |runner| {
            group.bench_function("mono_small", |b| {
                b.iter(|| black_box(runner.call_words(&[ITERS]).unwrap()));
            });
        },
    )
    .unwrap();
    with_loaded_mono_function(
        SOURCE,
        SourceKind::Move,
        addr,
        IdentStr::new("move_range_bench").unwrap(),
        IdentStr::new("shuffle_large").unwrap(),
        |runner| {
            group.bench_function("mono_large", |b| {
                b.iter(|| black_box(runner.call_words(&[ITERS]).unwrap()));
            });
        },
    )
    .unwrap();

    group.finish();
}

criterion_group!(benches, bench_move_range);
criterion_main!(benches);
