// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_logger::{Level, Logger};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_move_testing_utils::create_debugger;
use aptos_push_metrics::MetricsPusher;
pub use benchmark::BenchmarkCommand;
use clap::Parser;
pub use diff::DiffCommand;
pub use download::DownloadCommand;
pub use initialize::InitializeCommand;

mod benchmark;
mod diff;
mod download;
mod initialize;

pub(crate) fn init_logger_and_metrics(log_level: Level) {
    let mut logger = Logger::new();
    logger.level(log_level);
    logger.init();

    let _mp = MetricsPusher::start(vec![]);
}

pub(crate) fn build_debugger(
    rest_endpoint: String,
    api_key: Option<String>,
) -> anyhow::Result<AptosDebugger> {
    create_debugger(&rest_endpoint, api_key)
}

#[derive(Parser)]
pub struct RestAPI {
    #[clap(
        long,
        help = "Fullnode's REST API query endpoint, e.g., https://api.mainnet.aptoslabs.com/v1 \
                for mainnet"
    )]
    rest_endpoint: String,

    #[clap(
        long,
        help = "Optional API key to increase HTTP request rate limit quota"
    )]
    api_key: Option<String>,
}
