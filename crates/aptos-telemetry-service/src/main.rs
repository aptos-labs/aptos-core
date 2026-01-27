#![forbid(unsafe_code)]

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_telemetry_service::AptosTelemetryServiceArgs;
use clap::Parser;

#[tokio::main]
async fn main() {
    aptos_logger::Logger::new().init();
    AptosTelemetryServiceArgs::parse().run().await;
}
