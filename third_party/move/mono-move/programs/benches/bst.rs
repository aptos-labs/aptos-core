// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

const N_OPS: u64 = 5000;
const KEY_RANGE: u64 = 2500;
const SEED: u64 = 42;

fn bench_bst(c: &mut Criterion) {
    use mono_move_programs::{
        bst::{generate_ops, micro_op_bst, move_bytecode_bst, move_stdlib_vector, native_run_ops},
        testing,
    };
    use mono_move_runtime::InterpreterContext;

    let ops = generate_ops(N_OPS, KEY_RANGE, SEED);

    // -- native & micro_op -------------------------------------------------
    {
        let mut group = c.benchmark_group("bst");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        group.bench_function("native", |b| {
            b.iter(|| native_run_ops(black_box(&ops)));
        });

        let (mut functions, descriptors) = micro_op_bst();
        mono_move_programs::resolve_calls(&mut functions);
        group.bench_function("micro_op", |b| {
            b.iter_batched(
                || {
                    let mut ctx = InterpreterContext::new(&functions, &descriptors, 6);
                    let vec_ptr = ctx
                        .alloc_u64_vec(mono_move_runtime::DescriptorId(0), &ops)
                        .unwrap();
                    ctx.set_root_arg(0, &vec_ptr.to_le_bytes());
                    ctx
                },
                |mut ctx| ctx.run().unwrap(),
                BatchSize::SmallInput,
            );
        });
        group.finish();
    }

    // -- move_vm -----------------------------------------------------------
    {
        let mut group = c.benchmark_group("bst");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let module = move_bytecode_bst();
        let vector_module = move_stdlib_vector();
        let serialized_arg = testing::arg_vec_u64(&ops);
        testing::with_loaded_move_function_with_deps(
            &module,
            &[&vector_module],
            "run_ops",
            |env| {
                group.bench_function("move_vm", |b| {
                    b.iter(|| {
                        let result = env.run(vec![serialized_arg.clone()]);
                        black_box(result)
                    });
                });
            },
        );
        group.finish();
    }
}

criterion_group!(benches, bench_bst);
criterion_main!(benches);
