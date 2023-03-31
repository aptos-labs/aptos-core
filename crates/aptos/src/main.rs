// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Aptos is a one stop tool for operations, debugging, and other operations with the blockchain

#![forbid(unsafe_code)]

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use aptos::{config, move_tool, update::helpers::check_if_update_required, Tool};
use clap::Parser;
use std::{
    process::exit,
    time::{Duration, SystemTime},
};

const ONE_DAY: Duration = Duration::from_secs(60 * 60 * 24);

#[tokio::main]
async fn main() {
    // Check for updates
    check_for_update().await;

    // Register hooks
    move_tool::register_package_hooks();
    // Run the corresponding tools
    let result = Tool::parse().execute().await;
    // At this point, we'll want to print and determine whether to exit for an error code
    match result {
        Ok(inner) => println!("{}", inner),
        Err(inner) => {
            println!("{}", inner);
            exit(1);
        },
    }
}

/// Check if there needs to be an update, ignoring errors so the CLI still works, updates aren't required
pub async fn check_for_update() {
    if let Ok(mut global_config) = config::GlobalConfig::load() {
        // If this flag is set, skip the update check
        if global_config.skip_update_check.unwrap_or(false) {
            return;
        }
        let last_checked_time =
            Duration::from_secs(global_config.last_update_check_time.unwrap_or(0));
        if let Ok(now) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            if (now.saturating_sub(last_checked_time)) > ONE_DAY {
                if let Ok(Ok(update_info)) = tokio::task::spawn_blocking(move || {
                    check_if_update_required("aptos-labs", "aptos-core")
                })
                .await
                {
                    if update_info.update_required {
                        eprintln!("== UPDATE NOTICE ==\nA new version of the Aptos CLI is out {}, please run `aptos update` to update\n", update_info.latest_version);
                    }
                }

                // We've checked so, we will update last update check_time
                global_config.last_update_check_time = Some(now.as_secs());
                let _ = global_config.save();
            }
        }
    }
}
