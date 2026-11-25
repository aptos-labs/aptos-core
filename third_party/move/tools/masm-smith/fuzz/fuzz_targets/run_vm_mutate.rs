// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fuzz target: runs encoded tests on V1 (see [`vm_oracle`]), with mutation and crossover that
//! keep them verifiable (see [`mutation`]). Bytes that do not decode as a test seed the
//! generator instead. Inputs that reach new transaction outcomes are kept even without new code
//! coverage (see [`state_feedback`]).
//!
//! Run with `-max_len` above the default 4096: encoded tests reach several KiB.

#![no_main]

#[path = "common/mutation.rs"]
mod mutation;
#[path = "common/state_feedback.rs"]
mod state_feedback;
#[path = "common/stats.rs"]
mod stats;
#[path = "common/vm_oracle.rs"]
mod vm_oracle;

use libfuzzer_sys::{fuzz_crossover, fuzz_mutator, fuzz_target};

fuzz_target!(|data: &[u8]| {
    let Some(test) = mutation::test_from_input(data, &vm_oracle::GEN_CONFIG) else {
        return;
    };
    vm_oracle::run(&test);
});

fuzz_mutator!(|data: &mut [u8], size: usize, max_size: usize, seed: u32| {
    mutation::mutate_input(data, size, max_size, seed, &vm_oracle::GEN_CONFIG)
});

fuzz_crossover!(|data1: &[u8], data2: &[u8], out: &mut [u8], seed: u32| {
    mutation::crossover_inputs(data1, data2, out, seed, &vm_oracle::GEN_CONFIG)
});
