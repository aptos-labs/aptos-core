// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Aptos is a one stop tool for operations, debugging, and other operations with the blockchain

#![forbid(unsafe_code)]

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use aptos::{move_tool, update::get_update_message, Tool};
use aptos_logger::debug;
use clap::Parser;
use std::{process::exit, time::Duration};
use tokio::runtime::Runtime;

fn main() {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(_main());
    // We know when _main ends there are no tasks left running, so this should be safe.
    // This is necessary for now because self_update doesn't allow you to set a timeout.
    // https://github.com/jaemk/self_update/issues/100
    runtime.shutdown_timeout(Duration::from_secs(0));
}

async fn _main() {
    // Spin off a thread to check if the CLI needs to be updated. We take note of when
    // we started this thread so we can give up after a timeout.
    let update_check_handle =
        tokio::task::spawn_blocking(move || get_update_message("aptos-labs", "aptos-core"));

    // Register hooks
    move_tool::register_package_hooks();

    // Run the corresponding tools
    let result = Tool::parse().execute().await;

    // Wait for either the update check to complete or the timeout to happen. We only
    // print anything if the update check worked, otherwise we just print nothing and
    // move on. If RUST_LOG=debug is set, the inner function will log some helpful
    // information if there is a problem.
    match tokio::time::timeout(Duration::from_millis(1000), update_check_handle).await {
        Ok(message) => {
            if let Ok(Some(message)) = message {
                println!("{}", message);
            }
        },
        Err(e) => {
            debug!("Self-update check timed out: {:#}", e);
        },
    }

    // At this point, we'll want to print and determine whether to exit for an error code
    match result {
        Ok(inner) => println!("{}", inner),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        },
    }
}
