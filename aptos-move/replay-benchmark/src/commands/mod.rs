// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::{Level, Logger};
// use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_push_metrics::MetricsPusher;
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_validator_interface::{AptosValidatorInterface, RestDebuggerInterface};
pub use benchmark::BenchmarkCommand;
use clap::Parser;
pub use diff::DiffCommand;
pub use download::DownloadCommand;
pub use initialize::InitializeCommand;
use std::sync::Arc;
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
) -> anyhow::Result<Arc<dyn AptosValidatorInterface + Send>> {
    let builder = Client::builder(AptosBaseUrl::Custom(Url::parse(&rest_endpoint)?));
    let client = if let Some(api_key) = api_key {
        builder.api_key(&api_key)?.build()
    } else {
        builder.build()
    };
    Ok(Arc::new(RestDebuggerInterface::new(client)))
    //AptosDebugger::rest_client(client)
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
