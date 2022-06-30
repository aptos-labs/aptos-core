// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use super::common::ServerArgs;
use crate::{
    configuration::{
        NodeAddress, DEFAULT_API_PORT_STR, DEFAULT_METRICS_PORT_STR, DEFAULT_NOISE_PORT_STR,
    },
    metric_collector::ReqwestMetricCollector,
    server::api::PreconfiguredNode,
};
use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use poem::{listener::TcpListener, Route, Server};
use url::Url;

use super::{
    api::{build_openapi_service, Api},
    configurations_manager::build_server_with_blocking_runner,
};

#[derive(Clone, Debug, Parser)]
pub struct Run {
    #[clap(flatten)]
    server_args: ServerArgs,

    /// File paths leading to baseline node configurations.
    #[structopt(long, parse(from_os_str), required = true, min_values = 1)]
    pub baseline_node_config_paths: Vec<PathBuf>,

    /// If this is given, the user will be able to call the check_preconfigured_node
    /// endpoint, which takes no target, instead using this as the target. If
    /// allow_preconfigured_test_node_only is set, only the check_preconfigured_node
    /// endpoint will work, the node will not respond to requests for other nodes.
    #[clap(long)]
    pub target_node_url: Option<Url>,

    /// The metrics port for the target node.
    #[clap(long, requires = "target-node-url", default_value = &DEFAULT_METRICS_PORT_STR)]
    pub target_metrics_port: u16,

    /// The API port for the target node.
    #[clap(long, requires = "target-node-url", default_value = &DEFAULT_API_PORT_STR)]
    pub target_api_port: u16,

    /// The port over which validator nodes can talk to the target node.
    #[clap(long, requires = "target-node-url", default_value = &DEFAULT_NOISE_PORT_STR)]
    pub target_noise_port: u16,

    /// If a test node is preconfigured, you can set this to prevent the server
    /// from responding to requests for any node but that one.
    #[clap(long)]
    pub allow_preconfigured_test_node_only: bool,
}

pub async fn run(args: Run) -> Result<()> {
    let configurations_manager =
        build_server_with_blocking_runner(&args.baseline_node_config_paths)
            .await
            .context("Failed to build baseline node configurations")?;

    info!(
        "Running with the following configuration: {:#?}",
        configurations_manager.configurations
    );

    let preconfigured_test_node = match args.target_node_url {
        Some(ref url) => {
            let node_address = NodeAddress {
                url: url.clone(),
                api_port: args.target_api_port,
                metrics_port: args.target_metrics_port,
                noise_port: args.target_noise_port,
            };
            let metric_collector =
                ReqwestMetricCollector::new(node_address.url.clone(), node_address.metrics_port);
            Some(PreconfiguredNode {
                node_address,
                metric_collector,
            })
        }
        None => None,
    };

    let api = Api {
        configurations_manager,
        preconfigured_test_node,
        allow_preconfigured_test_node_only: args.allow_preconfigured_test_node_only,
    };

    let api_endpoint = format!("/{}", args.server_args.api_path);
    let api_service = build_openapi_service(
        api,
        args.server_args.listen_address.clone(),
        &args.server_args.api_path,
    );
    let ui = api_service.swagger_ui();
    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    Server::new(TcpListener::bind((
        args.server_args
            .listen_address
            .host_str()
            .with_context(|| {
                format!(
                    "Failed to pull host from {}",
                    args.server_args.listen_address
                )
            })?,
        args.server_args.listen_address.port().unwrap(),
    )))
    .run(
        Route::new()
            .nest(api_endpoint, api_service)
            .nest("/docs", ui)
            .at("/spec_json", spec_json)
            .at("/spec_yaml", spec_yaml),
    )
    .await
    .map_err(anyhow::Error::msg)
}
