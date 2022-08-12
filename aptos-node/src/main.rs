// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_node::AptosNodeArgs;
use clap::Parser;

use tikv_jemallocator::Jemalloc;

#[global_allocator]
static ALLOC: Jemalloc = Jemalloc;

fn main() {
    AptosNodeArgs::parse().run()
}
