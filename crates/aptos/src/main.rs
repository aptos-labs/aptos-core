// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Aptos is a one stop tool for operations, debugging, and other operations with the blockchain

#![forbid(unsafe_code)]

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use aptos::{move_tool, Tool};
use clap::Parser;
use std::{process::exit, time::Duration};

fn main() {
    // Register hooks.
    move_tool::register_package_hooks();

    // Create a runtime.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // Run the corresponding tool.
    let result = runtime.block_on(Tool::parse().execute());

    // Shutdown the runtime with a timeout. We do this to make sure that we don't sit
    // here waiting forever waiting for tasks that sometimes don't want to exit on
    // their own (e.g. telemetry, containers spawned by the localnet, etc). abcd.
    runtime.shutdown_timeout(Duration::from_millis(50));

    match result {
        Ok(inner) => println!("{}", inner),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        },
    }
}
