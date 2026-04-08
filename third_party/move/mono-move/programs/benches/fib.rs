// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

const N: u64 = 25;

fn bench_fib(c: &mut Criterion) {
    use mono_move_programs::{
        fib::{micro_op_fib, move_bytecode_fib, native_fib},
        testing,
    };
    use mono_move_runtime::InterpreterContext;

    // -- native & micro_op (fast) -----------------------------------------
    {
        let mut group = c.benchmark_group("fib");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        group.bench_function("native", |b| {
            b.iter(|| black_box(native_fib(N)));
        });

        let (mut functions, descriptors) = micro_op_fib();
        mono_move_programs::resolve_calls(&mut functions);
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

    // -- move_vm (slow) ---------------------------------------------------
    {
        let mut group = c.benchmark_group("fib");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let module = move_bytecode_fib();
        testing::with_loaded_move_function(&module, "fib", |env| {
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

criterion_group!(benches, bench_fib);
criterion_main!(benches);
