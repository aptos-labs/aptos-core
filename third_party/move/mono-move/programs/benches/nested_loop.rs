// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

const N: u64 = 1000;

fn bench_nested_loop(c: &mut Criterion) {
    use mono_move_programs::{
        nested_loop::{micro_op_nested_loop, move_bytecode_nested_loop, native_nested_loop},
        testing,
    };
    use mono_move_runtime::InterpreterContext;

    // -- native & micro_op -------------------------------------------------
    {
        let mut group = c.benchmark_group("nested_loop");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        group.bench_function("native", |b| {
            b.iter(|| black_box(native_nested_loop(N)));
        });

        let (functions, descriptors) = micro_op_nested_loop();
        group.bench_function("micro_op", |b| {
            b.iter_batched(
                || {
                    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
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
        let mut group = c.benchmark_group("nested_loop");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let module = move_bytecode_nested_loop();
        testing::with_loaded_move_function(&module, "nested_loop", |env| {
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

criterion_group!(benches, bench_nested_loop);
criterion_main!(benches);
