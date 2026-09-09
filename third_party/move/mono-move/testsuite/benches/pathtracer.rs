// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// A pixel on the left wall level with the light, where a bounce reaches the
/// light often enough that the result is not a constant black.
const X: u64 = 168;
const Y: u64 = 574;
const SPP: u64 = 16;
const MAX_DEPTH: u64 = 6;
const SEED: u64 = 1;

fn bench_pathtracer(c: &mut Criterion) {
    use mono_move_testsuite::{
        programs::{
            pathtracer::{move_bytecode_pathtracer, SOURCE},
            testing,
        },
        with_loaded_mono_function, SourceKind,
    };
    use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

    // -- mono-move pipeline -----------------------------------------------
    let expected = {
        let mut group = c.benchmark_group("pathtracer");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let addr = AccountAddress::from_hex_literal("0xb0").unwrap();
        let expected = with_loaded_mono_function(
            SOURCE,
            SourceKind::Move,
            addr,
            IdentStr::new("pathtracer").unwrap(),
            IdentStr::new("bench_trace_pixel").unwrap(),
            |runner| {
                let expected = runner.call_words(&[X, Y, SPP, MAX_DEPTH, SEED]).unwrap();
                group.bench_function("mono", |b| {
                    b.iter(|| black_box(runner.call_words(&[X, Y, SPP, MAX_DEPTH, SEED]).unwrap()));
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
        let mut group = c.benchmark_group("pathtracer");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let (module, fixed) = move_bytecode_pathtracer();
        testing::with_loaded_move_function_with_deps(
            &module,
            &[&fixed],
            "bench_trace_pixel",
            |env| {
                let args = || {
                    vec![
                        testing::arg_u64(X),
                        testing::arg_u64(Y),
                        testing::arg_u64(SPP),
                        testing::arg_u64(MAX_DEPTH),
                        testing::arg_u64(SEED),
                    ]
                };
                let result = env.run(args());
                assert_eq!(testing::return_u64(&result), expected, "VMs disagree");

                group.bench_function("move_vm", |b| {
                    b.iter(|| {
                        let result = env.run(args());
                        black_box(testing::return_u64(&result))
                    });
                });
            },
        );
        group.finish();
    }
}

criterion_group!(benches, bench_pathtracer);
criterion_main!(benches);
