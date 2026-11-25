// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fuzz target: runs each test on V1 and on MonoMove and reports where they disagree (see
//! [`mono_oracle`]).
//!
//! Inputs are encoded tests, mutated as in `run_vm_mutate`. An input that is the text of a
//! transactional test (Move or assembly) runs as is, so corpus sources and reduced findings can
//! be replayed.
//!
//! Run with `-max_len` above the default 4096: encoded tests reach several KiB.

#![no_main]

// The `mono_allowlist` test reads the fields this target does not.
#[allow(dead_code)]
#[path = "common/mono_allowlist.rs"]
mod mono_allowlist;
#[path = "common/mono_oracle.rs"]
mod mono_oracle;
#[path = "common/mutation.rs"]
mod mutation;
#[path = "common/state_feedback.rs"]
mod state_feedback;
#[path = "common/stats.rs"]
mod stats;
// Shared with the single-VM targets, which use the parts this target does not.
#[allow(dead_code)]
#[path = "common/vm_oracle.rs"]
mod vm_oracle;

use libfuzzer_sys::{fuzz_crossover, fuzz_mutator, fuzz_target};

fuzz_target!(|data: &[u8]| {
    if let Some(text) = source_text(data) {
        mono_oracle::run_source(text, source_extension(text));
        return;
    }
    let Some(test) = mutation::test_from_input(data, &mono_oracle::GEN_CONFIG) else {
        return;
    };
    mono_oracle::run_test(&test);
});

fuzz_mutator!(|data: &mut [u8], size: usize, max_size: usize, seed: u32| {
    mutation::mutate_input(data, size, max_size, seed, &mono_oracle::GEN_CONFIG)
});

fuzz_crossover!(|data1: &[u8], data2: &[u8], out: &mut [u8], seed: u32| {
    mutation::crossover_inputs(data1, data2, out, seed, &mono_oracle::GEN_CONFIG)
});

/// `data` as a transactional test source, if it is one.
fn source_text(data: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(data).ok()?;
    let directive = text.lines().any(|line| line.starts_with("//#"));
    (text.starts_with("//") && directive).then_some(text)
}

/// The file extension of the source `text`: Move declares modules and scripts with braces,
/// assembly without.
fn source_extension(text: &str) -> &'static str {
    let declares_with_brace = text.lines().map(str::trim).any(|line| {
        (line.starts_with("module ") || line.starts_with("script")) && line.ends_with('{')
    });
    if declares_with_brace {
        "move"
    } else {
        "masm"
    }
}
