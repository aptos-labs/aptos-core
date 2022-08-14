#![forbid(unsafe_code)]

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_telemetry_service::AptosTelemetryServiceArgs;
use clap::Parser;

#[tokio::main]
#[tracing::instrument(skip_all, level = "trace")]
async fn main() {
    aptos_logger::Logger::new().init();
    AptosTelemetryServiceArgs::parse().run().await;
}
