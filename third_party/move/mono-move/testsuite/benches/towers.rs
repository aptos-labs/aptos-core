// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const DISKS: u64 = 16;
const ITERS: u64 = 1;

fn bench_towers(c: &mut Criterion) {
    use mono_move_testsuite::{
        programs::{
            testing,
            towers::{move_bytecode_towers, SOURCE},
        },
        with_loaded_mono_function, SourceKind,
    };
    use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

    // -- mono-move pipeline -----------------------------------------------
    let expected = {
        let mut group = c.benchmark_group("towers");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let addr = AccountAddress::from_hex_literal("0xb0").unwrap();
        let expected = with_loaded_mono_function(
            SOURCE,
            SourceKind::Move,
            addr,
            IdentStr::new("towers").unwrap(),
            IdentStr::new("bench_towers").unwrap(),
            |runner| {
                let expected = runner.call_words(&[DISKS, ITERS]).unwrap();
                group.bench_function("mono", |b| {
                    b.iter(|| black_box(runner.call_words(&[DISKS, ITERS]).unwrap()));
                });
                expected
            },
        )
        .unwrap();

        group.finish();
        expected
    };

    // -- move_vm ----------------------------------------------------------
    {
        let mut group = c.benchmark_group("towers");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let module = move_bytecode_towers();
        testing::with_loaded_move_function(&module, "bench_towers", |env| {
            let args = || vec![testing::arg_u64(DISKS), testing::arg_u64(ITERS)];
            let result = env.run(args());
            assert_eq!(testing::return_u64(&result), expected, "VMs disagree");

            group.bench_function("move_vm", |b| {
                b.iter(|| {
                    let result = env.run(args());
                    black_box(testing::return_u64(&result))
                });
            });
        });
        group.finish();
    }
}

criterion_group!(benches, bench_towers);
criterion_main!(benches);
