// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[path = "helpers.rs"]
mod helpers;

const N: u64 = 1_000_000;

fn bench_match_sum(c: &mut Criterion) {
    use mono_move_core::LocalExecutionContext;
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
        let (functions, descriptors) = micro_op_match_sum();
        let mut exec_ctx = LocalExecutionContext::unmetered();
        // TODO: hoist interpreter context setup out of the timed body.
        group.bench_function("micro_op", |b| {
            b.iter(|| {
                let mut ctx = InterpreterContext::new(&mut exec_ctx, &descriptors, &functions[0]);
                ctx.set_root_arg(0, &N.to_le_bytes());
                ctx.run().unwrap();
                black_box(ctx.root_result());
            });
        });

        // with gas instrumentation
        let (functions, _) = micro_op_match_sum();
        let wrapped = functions.into_iter().map(Some).collect::<Vec<_>>();
        let functions_gas = helpers::gas_instrument(&wrapped);
        let mut exec_ctx = LocalExecutionContext::with_max_budget();
        // TODO: hoist interpreter context setup out of the timed body.
        group.bench_function("micro_op/gas", |b| {
            b.iter(|| {
                let mut ctx = InterpreterContext::new(
                    &mut exec_ctx,
                    &descriptors,
                    functions_gas[0].as_ref().unwrap(),
                );
                ctx.set_root_arg(0, &N.to_le_bytes());
                ctx.run().unwrap();
                black_box(ctx.root_result());
            });
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
