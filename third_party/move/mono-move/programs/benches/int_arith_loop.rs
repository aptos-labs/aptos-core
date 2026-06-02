// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Microbenchmark comparing the u64 specialized fast-path arith imm ops
//! ([`MicroOp::MulU64Imm`] etc.) against the unspecialized per-kind
//! variants ([`MicroOp::IntMul`] etc. carrying an [`IntOperand::ImmI64`]
//! rhs).
//!
//! Each iteration of the inner loop runs
//! [`mono_move_programs::int_arith_loop::ROUNDS_PER_ITER`] (= 30) rounds
//! of `acc = (acc * 31 + 17) % 1_000_003` — 90 body ops per iter, plus
//! ~2 loop-overhead ops. The two flavors share frame layout, loop
//! structure, and constants; only the body op variants differ, so the
//! per-bench delta isolates the dispatcher cost.
//!
//! [`MicroOp::MulU64Imm`]: mono_move_core::MicroOp::MulU64Imm
//! [`MicroOp::IntMul`]: mono_move_core::MicroOp::IntMul
//! [`IntOperand::ImmI64`]: mono_move_core::IntOperand::ImmI64

use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Loop iterations per bench call. With 90 body ops per iter, this gives
/// ~90k body ops per call — comfortably in the ms range even on the fast
/// path, so per-call setup (arena alloc, context construction) is
/// negligible.
const ITERS: u64 = 1_000;

fn bench_int_arith_loop(c: &mut Criterion) {
    use mono_move_programs::{
        int_arith_loop::{
            micro_op_i64_loop, micro_op_u64_loop, move_bytecode_int_arith_loop, native_i64_loop,
            native_u64_loop,
        },
        testing,
    };
    use mono_move_runtime::{InterpreterContext, LocalRuntimeContext};

    let mut group = c.benchmark_group("int_arith_loop");
    group
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(3));

    // -- native (control) --------------------------------------------------
    group.bench_function("native/u64", |b| {
        b.iter(|| black_box(native_u64_loop(ITERS)));
    });
    group.bench_function("native/i64", |b| {
        b.iter(|| black_box(native_i64_loop(ITERS)));
    });

    // -- micro_op u64 (specialized fast path) ------------------------------
    {
        let (functions, descriptors) = micro_op_u64_loop(false);
        let mut exec_ctx = LocalRuntimeContext::unmetered_with_descriptors(descriptors);
        // TODO: use `criterion::Bencher::iter_custom` to start/stop the timer
        // around the run, so context construction is excluded from the
        // measurement.
        group.bench_function("micro_op/u64", |b| {
            b.iter(|| {
                let mut ctx = InterpreterContext::new(&mut exec_ctx, unsafe {
                    functions[0].as_ref_unchecked()
                });
                ctx.set_root_arg(0, &ITERS.to_le_bytes());
                ctx.run().unwrap();
                black_box(ctx.root_result());
            });
        });
    }

    // -- micro_op i64 (unspecialized tag-dispatched) -----------------------
    {
        let (functions, descriptors) = micro_op_i64_loop(false);
        let mut exec_ctx = LocalRuntimeContext::unmetered_with_descriptors(descriptors);
        // TODO: use `criterion::Bencher::iter_custom` to start/stop the timer
        // around the run, so context construction is excluded from the
        // measurement.
        group.bench_function("micro_op/i64", |b| {
            b.iter(|| {
                let mut ctx = InterpreterContext::new(&mut exec_ctx, unsafe {
                    functions[0].as_ref_unchecked()
                });
                ctx.set_root_arg(0, &ITERS.to_le_bytes());
                ctx.run().unwrap();
                black_box(ctx.root_result());
            });
        });
    }

    // -- micro_op u64 with gas instrumentation -----------------------------
    {
        let (functions_gas, descriptors) = micro_op_u64_loop(true);
        let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
        group.bench_function("micro_op/u64/gas", |b| {
            b.iter(|| {
                let mut ctx = InterpreterContext::new(&mut exec_ctx, unsafe {
                    functions_gas[0].as_ref_unchecked()
                });
                ctx.set_root_arg(0, &ITERS.to_le_bytes());
                ctx.run().unwrap();
                black_box(ctx.root_result());
            });
        });
    }

    // -- micro_op i64 with gas instrumentation -----------------------------
    {
        let (functions_gas, descriptors) = micro_op_i64_loop(true);
        let mut exec_ctx = LocalRuntimeContext::with_max_budget(descriptors);
        group.bench_function("micro_op/i64/gas", |b| {
            b.iter(|| {
                let mut ctx = InterpreterContext::new(&mut exec_ctx, unsafe {
                    functions_gas[0].as_ref_unchecked()
                });
                ctx.set_root_arg(0, &ITERS.to_le_bytes());
                ctx.run().unwrap();
                black_box(ctx.root_result());
            });
        });
    }

    group.finish();

    // -- move_vm (production interpreter) ---------------------------------
    //
    // Compares against the production Move VM running the same loop. We
    // shrink sample size since the Move VM is significantly slower per op
    // than the mono-move interpreter.
    {
        let mut group = c.benchmark_group("int_arith_loop");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let module = move_bytecode_int_arith_loop();
        testing::with_loaded_move_function(&module, "u64_loop", |env| {
            group.bench_function("move_vm/u64", |b| {
                b.iter(|| {
                    let result = env.run(vec![testing::arg_u64(ITERS)]);
                    black_box(testing::return_u64(&result))
                });
            });
        });
        testing::with_loaded_move_function(&module, "i64_loop", |env| {
            group.bench_function("move_vm/i64", |b| {
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
