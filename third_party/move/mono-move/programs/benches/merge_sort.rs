// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

const N: u64 = 1000;

fn bench_merge_sort(c: &mut Criterion) {
    use mono_move_programs::{
        merge_sort::{
            micro_op_merge_sort, move_bytecode_merge_sort, native_merge_sort, shuffled_range,
        },
        testing,
    };
    use mono_move_runtime::InterpreterContext;

    let input = shuffled_range(N, 42);

    // -- native & micro_op -------------------------------------------------
    {
        let mut group = c.benchmark_group("merge_sort");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        group.bench_function("native", |b| {
            b.iter_batched(
                || input.clone(),
                |mut v| {
                    native_merge_sort(&mut v);
                    black_box(v)
                },
                BatchSize::SmallInput,
            );
        });

        let (functions, descriptors) = micro_op_merge_sort();
        group.bench_function("micro_op", |b| {
            b.iter_batched(
                || {
                    let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
                    let vec_ptr = ctx
                        .alloc_u64_vec(mono_move_runtime::DescriptorId(0), &input)
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
        let mut group = c.benchmark_group("merge_sort");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let module = move_bytecode_merge_sort();
        let serialized_arg = testing::arg_vec_u64(&input);
        testing::with_loaded_move_function(&module, "merge_sort", |env| {
            group.bench_function("move_vm", |b| {
                b.iter(|| {
                    let result = env.run(vec![serialized_arg.clone()]);
                    black_box(result.return_values[0].0.len())
                });
            });
        });
        group.finish();
    }
}

criterion_group!(benches, bench_merge_sort);
criterion_main!(benches);
