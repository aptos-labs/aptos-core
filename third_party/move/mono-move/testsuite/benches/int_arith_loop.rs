// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Microbenchmark isolating arithmetic dispatch cost: `u64_loop` and
//! `i64_loop` run the same loop, so the u64 (specialized) vs i64
//! (unspecialized) delta is the per-op dispatch difference.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const ITERS: u64 = 1_000;

fn bench_int_arith_loop(c: &mut Criterion) {
    use mono_move_testsuite::{
        programs::{
            int_arith_loop::{
                move_bytecode_int_arith_loop, native_i64_loop, native_u64_loop, SOURCE,
            },
            testing,
        },
        with_loaded_mono_function, SourceKind,
    };
    use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

    let addr = AccountAddress::from_hex_literal("0x1").unwrap();

    // -- native (control) + mono-move pipeline ----------------------------
    {
        let mut group = c.benchmark_group("int_arith_loop");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        group.bench_function("native_u64", |b| {
            b.iter(|| black_box(native_u64_loop(ITERS)));
        });
        group.bench_function("native_i64", |b| {
            b.iter(|| black_box(native_i64_loop(ITERS)));
        });

        with_loaded_mono_function(
            SOURCE,
            SourceKind::Move,
            addr,
            IdentStr::new("int_arith_loop").unwrap(),
            IdentStr::new("u64_loop").unwrap(),
            |runner| {
                group.bench_function("mono_u64", |b| {
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
                group.bench_function("mono_i64", |b| {
                    // Same 8 bytes; reinterpret as i64.
                    b.iter(|| black_box(runner.call_words(&[ITERS]).unwrap() as i64));
                });
            },
        )
        .unwrap();

        group.finish();
    }

    // -- move_vm ----------------------------------------------------------
    {
        let mut group = c.benchmark_group("int_arith_loop");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

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
}

criterion_group!(benches, bench_int_arith_loop);
criterion_main!(benches);
