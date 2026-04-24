// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

#[path = "helpers.rs"]
mod helpers;

const N: u64 = 1_000_000;

fn bench_match_sum(c: &mut Criterion) {
    use mono_move_core::NoopTransactionContext;
    use mono_move_gas::{NoOpGasMeter, SimpleGasMeter};
    use mono_move_programs::{
        match_sum::{micro_op_match_sum, move_bytecode_match_sum, native_match_sum},
        testing,
    };
    use mono_move_runtime::InterpreterContext;

    // -- native & micro_op -------------------------------------------------
    {
        let mut group = c.benchmark_group("match_sum");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        group.bench_function("native", |b| {
            b.iter(|| black_box(native_match_sum(N)));
        });

        // plain (no gas instrumentation)
        let (functions, descriptors, _arena) = micro_op_match_sum();
        let txn_ctx = NoopTransactionContext;
        group.bench_function("micro_op", |b| {
            b.iter_batched(
                || {
                    let mut ctx =
                        InterpreterContext::new(&txn_ctx, &descriptors, NoOpGasMeter, unsafe {
                            functions[0].as_ref_unchecked()
                        });
                    ctx.set_root_arg(0, &N.to_le_bytes());
                    ctx
                },
                |mut ctx| {
                    ctx.run().unwrap();
                    black_box(ctx.root_result())
                },
                BatchSize::SmallInput,
            );
        });

        // with gas instrumentation
        let (functions, _, _arena) = micro_op_match_sum();
        let wrapped = functions.iter().map(|f| Some(*f)).collect::<Vec<_>>();
        // SAFETY: Exclusive access during bench setup; arena is alive.
        let (functions_gas, _arena) = unsafe { helpers::gas_instrument(&wrapped) };
        group.bench_function("micro_op/gas", |b| {
            b.iter_batched(
                || {
                    let gas_meter = SimpleGasMeter::new(u64::MAX);
                    let mut ctx =
                        InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
                            functions_gas[0].unwrap().as_ref_unchecked()
                        });
                    ctx.set_root_arg(0, &N.to_le_bytes());
                    ctx
                },
                |mut ctx| {
                    ctx.run().unwrap();
                    black_box(ctx.root_result())
                },
                BatchSize::SmallInput,
            );
        });

        group.finish();
    }

    // -- move_vm -----------------------------------------------------------
    {
        let mut group = c.benchmark_group("match_sum");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

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
}

criterion_group!(benches, bench_match_sum);
criterion_main!(benches);
