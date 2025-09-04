// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_logger::{Level, Logger};
use velor_move_debugger::velor_debugger::VelorDebugger;
use velor_push_metrics::MetricsPusher;
use velor_rest_client::{VelorBaseUrl, Client};
pub use benchmark::BenchmarkCommand;
use clap::Parser;
pub use diff::DiffCommand;
pub use download::DownloadCommand;
pub use initialize::InitializeCommand;
use url::Url;

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
) -> anyhow::Result<VelorDebugger> {
    let builder = Client::builder(VelorBaseUrl::Custom(Url::parse(&rest_endpoint)?));
    let client = if let Some(api_key) = api_key {
        builder.api_key(&api_key)?.build()
    } else {
        builder.build()
    };
    VelorDebugger::rest_client(client)
}

#[derive(Parser)]
pub struct RestAPI {
    #[clap(
        long,
        help = "Fullnode's REST API query endpoint, e.g., https://api.mainnet.velorlabs.com/v1 \
                for mainnet"
    )]
    rest_endpoint: String,

    #[clap(
        long,
        help = "Optional API key to increase HTTP request rate limit quota"
    )]
    api_key: Option<String>,
}
