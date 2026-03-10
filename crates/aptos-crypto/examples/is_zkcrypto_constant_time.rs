// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub use aptos_crypto::constant_time;
use dudect_bencher::ctbench_main;

ctbench_main!(constant_time::zkcrypto_scalar_mul::run_bench);
