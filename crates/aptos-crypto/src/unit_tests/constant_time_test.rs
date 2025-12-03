// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::constant_time;
use dudect_bencher::ctbench::{run_bench, BenchName};
use more_asserts::assert_le;
use num_traits::ToPrimitive;

#[test]
#[ignore]
/// WARNING: This is marked as "ignored" because unit tests are typically run in debug mode, and we
/// would need this to run in release mode to make sure the dudect framework's statistical measurements
/// are meaningful.
///
/// Nonetheless, we wrote this test to serve as an example for how to call the dudect framework
/// manually, without using the macros that would generate a `main` function, which would not work
/// if we want to run these tests in some other `main` function (like the pepper service).
///
/// To run this test properly, do:
///
///    cargo test --release test_blstrs_fixed_base_g1_scalar_mul_is_constant_time -- --ignored --nocapture
///
fn test_blstrs_fixed_base_g1_scalar_mul_is_constant_time() {
    let ct_summary = run_bench(
        &BenchName("blstrs_scalar_mul_fixed_base"),
        constant_time::blstrs_scalar_mul::run_bench_with_fixed_bases,
        None,
    )
    .1;

    eprintln!("{:?}", ct_summary);

    let max_t = ct_summary
        .max_t
        .abs()
        .to_i64()
        .expect("Floating point arithmetic went awry.");
    assert_le!(max_t, 5);
}

#[test]
#[ignore]
/// To run this test properly, do:
///
///    cargo test --release test_blstrs_random_base_g1_scalar_mul_is_constant_time -- --ignored --nocapture
///
fn test_blstrs_random_base_g1_scalar_mul_is_constant_time() {
    let ct_summary = run_bench(
        &BenchName("blstrs_scalar_mul_random_base"),
        constant_time::blstrs_scalar_mul::run_bench_with_random_bases,
        None,
    )
    .1;

    eprintln!("{:?}", ct_summary);

    let max_t = ct_summary
        .max_t
        .abs()
        .to_i64()
        .expect("Floating point arithmetic went awry.");
    assert_le!(max_t, 5);
}
