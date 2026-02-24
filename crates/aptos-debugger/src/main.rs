// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_debugger::Cmd;
use aptos_logger::{Level, Logger};
use aptos_push_metrics::MetricsPusher;
use clap::Parser;

#[cfg(unix)]
aptos_jemalloc::setup_jemalloc!();

#[tokio::main]
async fn main() -> Result<()> {
    Logger::new().level(Level::Info).init();
    let _mp = MetricsPusher::start(vec![]);

    Cmd::parse().run().await
}
