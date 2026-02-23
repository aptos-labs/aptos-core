// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Aptos is a one stop tool for operations, debugging, and other operations with the blockchain

#![forbid(unsafe_code)]

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use aptos::Tool;
use clap::Parser;
use std::{process::exit, time::Duration};

/// Telemetry callback that bridges `aptos-cli-common`'s pluggable telemetry
/// to the full Aptos CLI's telemetry subsystem.
struct CliTelemetry;

impl aptos_cli_common::TelemetryCallback for CliTelemetry {
    fn send_event(&self, command_name: &str, latency_secs: f64, success: bool) {
        let build_info = aptos::common::utils::cli_build_information();
        let command = command_name.to_string();
        let latency = Duration::from_secs_f64(latency_secs);
        let error: Option<&'static str> = if success { None } else { Some("Error") };

        // Spawn the async telemetry send as a fire-and-forget task.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                aptos_telemetry::cli_metrics::send_cli_telemetry_event(
                    build_info, command, latency, success, error,
                )
                .await;
            });
        }
    }

    fn is_disabled(&self) -> bool {
        aptos_telemetry::service::telemetry_is_disabled()
    }
}

fn main() {
    // Register hooks.
    aptos_move_cli::register_package_hooks();

    // Register the telemetry callback so `aptos-cli-common`'s `to_common_result`
    // reports CLI metrics through the full telemetry subsystem.
    aptos_cli_common::register_telemetry(Box::new(CliTelemetry));

    // Create a runtime.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

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
