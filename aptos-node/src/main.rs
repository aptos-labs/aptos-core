// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![deny(unsafe_code)]

use aptos_node::{utils::ERROR_MSG_BAD_FEATURE_FLAGS, AptosNodeArgs};
use clap::Parser;

#[cfg(unix)]
aptos_jemalloc::setup_jemalloc!();

fn main() {
    // Remap .text onto 2MB huge pages to reduce iTLB misses.
    // Must happen before spawning threads.
    #[cfg(target_os = "linux")]
    match aptos_hugify::hugify_process_text() {
        Ok(n) => eprintln!("hugify: remapped {n} x 2MB pages"),
        Err(e) => panic!("hugify failed: {e}"),
    }

    // Check that we are not including any Move test natives
    aptos_vm::natives::assert_no_test_natives(ERROR_MSG_BAD_FEATURE_FLAGS);

    // Start the node
    AptosNodeArgs::parse().run()
}
