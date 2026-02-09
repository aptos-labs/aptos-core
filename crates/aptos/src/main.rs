// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Aptos is a one stop tool for operations, debugging, and other operations with the blockchain

#![forbid(unsafe_code)]

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use aptos::{move_tool, update::version_check, Tool};
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

    // Spawn a background version check (non-blocking, at most once per day).
    // This reads a local cache file and prints a notice to stderr if a newer
    // version is available. A background task refreshes the cache if stale.
    let _version_check_handle =
        runtime.block_on(async { version_check::check_for_update_and_notify() });

    // Run the corresponding tool.
    let result = runtime.block_on(Tool::parse().execute());

    // Shutdown the runtime with a timeout. We do this to make sure that we don't sit
    // here waiting forever waiting for tasks that sometimes don't want to exit on
    // their own (e.g. telemetry, containers spawned by the localnet, etc).
    runtime.shutdown_timeout(Duration::from_millis(50));

    match result {
        Ok(inner) => println!("{}", inner),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        },
    }
}
