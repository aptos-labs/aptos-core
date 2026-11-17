// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fuzz target: generates a fresh test from each input and runs it on V1 (see [`vm_oracle`]).
//!
//! `MASM_SMITH_STATS=<file>` appends one JSON line per input with the task count and each
//! failing task's kind, status, and message.

#![no_main]

#[path = "common/state_feedback.rs"]
mod state_feedback;
#[path = "common/stats.rs"]
mod stats;
#[path = "common/vm_oracle.rs"]
mod vm_oracle;

use arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use masm_smith::generate;

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let Ok(test) = generate(&mut u, &vm_oracle::GEN_CONFIG) else {
        return;
    };
    vm_oracle::run(&test);
});
