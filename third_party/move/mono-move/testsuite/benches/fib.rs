// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const N: u64 = 25;

fn bench_fib(c: &mut Criterion) {
    use mono_move_testsuite::{
        programs::{
            fib::{move_bytecode_fib, native_fib, SOURCE},
            testing,
        },
        with_loaded_mono_function, SourceKind,
    };
    use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

    // -- native (control) + mono-move pipeline ----------------------------
    {
        let mut group = c.benchmark_group("fib");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        group.bench_function("native", |b| {
            b.iter(|| black_box(native_fib(N)));
        });

        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        with_loaded_mono_function(
            SOURCE,
            SourceKind::Move,
            addr,
            IdentStr::new("fib").unwrap(),
            IdentStr::new("fib").unwrap(),
            |runner| {
                group.bench_function("mono", |b| {
                    b.iter(|| black_box(runner.call_words(&[N]).unwrap()));
                });
            },
        )
        .unwrap();

        group.finish();
    }

    // -- move_vm (production interpreter, slower → smaller sample) ---------
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
