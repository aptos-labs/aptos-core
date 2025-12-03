// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

mod config;
mod gather_metrics;
mod server;

pub use config::MetricsServerConfig;
pub use server::run_metrics_server;
