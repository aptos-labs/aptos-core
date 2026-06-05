// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const N_OPS: u64 = 5000;
const KEY_RANGE: u64 = 2500;
const SEED: u64 = 42;

fn bench_bst(c: &mut Criterion) {
    use mono_move_testsuite::{
        programs::{
            bst::{move_bytecode_bst, native_run_ops_checksum, SOURCE},
            testing,
        },
        with_loaded_mono_function, SourceKind,
    };
    use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

    // -- native (control) + mono-move pipeline ----------------------------
    {
        let mut group = c.benchmark_group("bst");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        group.bench_function("native", |b| {
            b.iter(|| black_box(native_run_ops_checksum(N_OPS, KEY_RANGE, SEED)));
        });

        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        with_loaded_mono_function(
            SOURCE,
            SourceKind::Move,
            addr,
            IdentStr::new("bst").unwrap(),
            IdentStr::new("run_ops_checksum").unwrap(),
            |runner| {
                group.bench_function("mono", |b| {
                    b.iter(|| black_box(runner.call_words(&[N_OPS, KEY_RANGE, SEED]).unwrap()));
                });
            },
        )
        .unwrap();

        group.finish();
    }

    // -- move_vm ----------------------------------------------------------
    {
        let mut group = c.benchmark_group("bst");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let module = move_bytecode_bst();
        testing::with_loaded_move_function(&module, "run_ops_checksum", |env| {
            group.bench_function("move_vm", |b| {
                b.iter(|| {
                    let result = env.run(vec![
                        testing::arg_u64(N_OPS),
                        testing::arg_u64(KEY_RANGE),
                        testing::arg_u64(SEED),
                    ]);
                    black_box(testing::return_u64(&result))
                });
            });
        });
        group.finish();
    }
}

criterion_group!(benches, bench_bst);
criterion_main!(benches);
