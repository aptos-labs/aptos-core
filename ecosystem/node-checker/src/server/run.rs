// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, convert::TryFrom, path::PathBuf};

use crate::{
    configuration::{
        self, read_configuration_from_file, NodeAddress, NodeConfiguration, DEFAULT_API_PORT_STR,
        DEFAULT_METRICS_PORT_STR, DEFAULT_NOISE_PORT_STR,
    },
    metric_collector::{MetricCollector, ReqwestMetricCollector},
    metric_evaluator::{build_evaluators, MetricsEvaluator},
    runner::{BlockingRunner, Runner},
};
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use log::debug;
use poem::{
    handler, http::StatusCode, listener::TcpListener, Error as PoemError, Result as PoemResult,
    Route, Server,
};
use poem_openapi::{payload::Json, OpenApi, OpenApiService};
use url::Url;

use super::{
    api::{build_openapi_service, Api},
    configurations_manager::build_server_with_blocking_runner_and_reqwest_metric_collector,
};

#[derive(Clone, Debug, Parser)]
pub struct Run {
    /// What address to listen on.
    #[clap(long, default_value = "http://0.0.0.0")]
    pub listen_address: Url,

    /// What port to listen on.
    #[clap(long, default_value = "20121")]
    pub listen_port: u16,

    /// File paths leading to baseline node configurations.
    #[structopt(long, parse(from_os_str), required = true, min_values = 1)]
    pub baseline_node_config_paths: Vec<PathBuf>,

    /// If this is given, the user will be able to call the check_preconfigured_node
    /// endpoint, which takes no target, instead using this as the target. If
    /// allow_preconfigured_test_node_only is set, only the check_preconfigured_node
    /// endpoint will work, the node will not respond to requests for other nodes.
    #[clap(long)]
    pub target_node_url: Option<Url>,

    // The following 3 arguments are only relevant if the user sets test_node_url.
    /// The metrics port for the target node.
    #[clap(long, default_value = &DEFAULT_METRICS_PORT_STR)]
    pub target_metrics_port: u16,

    /// The API port for the target node.
    #[clap(long, default_value = &DEFAULT_API_PORT_STR)]
    pub target_api_port: u16,

    /// The port over which validator nodes can talk to the target node.
    #[clap(long, default_value = &DEFAULT_NOISE_PORT_STR)]
    pub target_noise_port: u16,

    /// If a test node is preconfigured, you can set this to prevent the server
    /// from responding to requests for any node but that one.
    #[clap(long)]
    pub allow_preconfigured_test_node_only: bool,
}

pub async fn run(args: Run) -> Result<()> {
    let configurations_manager = build_server_with_blocking_runner_and_reqwest_metric_collector(
        &args.baseline_node_config_paths,
    )
    .await
    .context("Failed to build baseline node configurations")?;
    debug!(
        "Running with the following configuration: {:#?}",
        configurations_manager.configurations
    );

    let target_metric_collector = match args.target_node_url {
        Some(ref url) => Some(ReqwestMetricCollector::new(
            url.clone(),
            args.target_metrics_port,
        )),
        None => None,
    };

    let api = Api {
        configurations_manager,
        target_metric_collector,
        allow_preconfigured_test_node_only: args.allow_preconfigured_test_node_only,
    };

    let api_service =
        build_openapi_service(api, args.listen_address.clone(), args.listen_port, None);
    let ui = api_service.swagger_ui();
    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    debug!("Successfully built API");

    Server::new(TcpListener::bind((
        args.listen_address
            .host_str()
            .with_context(|| format!("Failed to pull host from {}", args.listen_address))?,
        args.listen_port,
    )))
    .run(
        Route::new()
            .nest("/", root)
            .nest("/api", api_service)
            .nest("/docs", ui)
            .at("/spec_json", spec_json)
            .at("/spec_yaml", spec_yaml),
    )
    .await
    .map_err(anyhow::Error::msg)
}

#[handler]
fn root() -> String {
    "Hello World!".to_string()
}
