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
use aptos_crypto::{x25519, ValidCryptoMaterialStringExt};
use clap::Parser;
use log::info;
use poem::{http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server};
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
    #[clap(
        long,
        required = true,
        min_values = 1,
        use_value_delimiter = true,
        parse(from_os_str)
    )]
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

    /// Public key for the node. This is used for the HandshakeEvaluator.
    /// If that evaluator is not enabled, this is not necessary.
    #[clap(long, requires = "target-node-url", value_parser = x25519::PublicKey::from_encoded_string)]
    pub target_public_key: Option<x25519::PublicKey>,

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
            let node_address = NodeAddress::new(url.clone())
                .api_port(args.target_api_port)
                .metrics_port(args.target_metrics_port)
                .noise_port(args.target_noise_port)
                .public_key(args.target_public_key);
            let metric_collector = ReqwestMetricCollector::new(node_address.clone());
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
    let api_service = build_openapi_service(api, args.server_args.clone());
    let ui = api_service.swagger_ui();
    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    let cors = Cors::new().allow_methods(vec![Method::GET]);

    Server::new(TcpListener::bind((
        args.server_args.listen_address,
        args.server_args.listen_port,
    )))
    .run(
        Route::new()
            .nest(api_endpoint, api_service)
            .nest("/spec", ui)
            .at("/spec.json", spec_json)
            .at("/spec.yaml", spec_yaml)
            .with(cors),
    )
    .await
    .map_err(anyhow::Error::msg)
}
