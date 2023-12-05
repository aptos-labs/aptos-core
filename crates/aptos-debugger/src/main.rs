// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_debugger::Cmd;
use aptos_logger::{Level, Logger};
use aptos_push_metrics::MetricsPusher;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    Logger::new().level(Level::Info).init();
    let _mp = MetricsPusher::start(vec![]);

    Cmd::parse().run().await
}
