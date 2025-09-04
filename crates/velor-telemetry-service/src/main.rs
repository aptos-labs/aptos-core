#![forbid(unsafe_code)]

// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_telemetry_service::VelorTelemetryServiceArgs;
use clap::Parser;

#[tokio::main]
async fn main() {
    velor_logger::Logger::new().init();
    VelorTelemetryServiceArgs::parse().run().await;
}
