// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_node::AptosNodeArgs;
use clap::Parser;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() {
    // Check that we are not including any Move test natives
    aptos_vm::natives::assert_no_test_natives();
    AptosNodeArgs::parse().run()
}
