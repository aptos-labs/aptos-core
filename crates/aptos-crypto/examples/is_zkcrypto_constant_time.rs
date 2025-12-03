// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

pub use aptos_crypto::constant_time;
use dudect_bencher::ctbench_main;

ctbench_main!(constant_time::zkcrypto_scalar_mul::run_bench);
