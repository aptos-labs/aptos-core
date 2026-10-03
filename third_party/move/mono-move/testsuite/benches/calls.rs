// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compares same-module, cross-module, and closure calls in matching loops.
//! Same-module calls skip the reentrancy check; the other calls run it.
//! Timing differences include both reentrancy checks and dispatch overhead.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const ITERS: u64 = 10_000;

fn bench_calls(c: &mut Criterion) {
    use mono_move_testsuite::{
        programs::{
            calls::{move_bytecode_calls, ENTRIES, MODULE, SOURCE},
            testing,
        },
        with_loaded_mono_function, SourceKind,
    };
    use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

    let addr = AccountAddress::from_hex_literal("0x1").unwrap();

    // -- mono-move pipeline ------------------------------------------------
    {
        let mut group = c.benchmark_group("calls");
        group
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        for entry in ENTRIES {
            with_loaded_mono_function(
                SOURCE,
                SourceKind::Move,
                addr,
                IdentStr::new(MODULE).unwrap(),
                IdentStr::new(entry).unwrap(),
                |runner| {
                    group.bench_function(format!("mono_{entry}"), |b| {
                        b.iter(|| black_box(runner.call_words(&[ITERS]).unwrap()));
                    });
                },
            )
            .unwrap();
        }

        group.finish();
    }

    // -- move_vm ----------------------------------------------------------
    {
        let mut group = c.benchmark_group("calls");
        group
            .sample_size(10)
            .warm_up_time(std::time::Duration::from_secs(1))
            .measurement_time(std::time::Duration::from_secs(3));

        let (module, helper) = move_bytecode_calls();
        for entry in ENTRIES {
            testing::with_loaded_move_function_with_deps(&module, &[&helper], entry, |env| {
                group.bench_function(format!("move_vm_{entry}"), |b| {
                    b.iter(|| {
                        let result = env.run(vec![testing::arg_u64(ITERS)]);
                        black_box(testing::return_u64(&result))
                    });
                });
            });
        }
        group.finish();
    }
}

criterion_group!(benches, bench_calls);
criterion_main!(benches);
