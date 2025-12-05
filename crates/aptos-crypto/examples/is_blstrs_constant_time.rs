// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub use aptos_crypto::constant_time;
use dudect_bencher::ctbench_main;

// Note: This runs the "fixed base" test. You'd need another Rust file to run the "random base" test.
ctbench_main!(constant_time::blstrs_scalar_mul::run_bench_with_fixed_bases);
