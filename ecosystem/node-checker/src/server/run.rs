// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    api::{build_openapi_service, Api},
    build::build_baseline_configuration_runners,
    common::ServerArgs,
};
use anyhow::{Context, Result};
use velor_logger::info;
use clap::Parser;
use poem::{http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server};
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
pub struct Run {
    #[clap(flatten)]
    server_args: ServerArgs,

    /// File paths leading to baseline configurations.
    #[clap(
        long,
        required = true,
        num_args = 1..,
        use_value_delimiter = true,
        value_parser
    )]
    pub baseline_config_paths: Vec<PathBuf>,
}

pub async fn run(args: Run) -> Result<()> {
    let baseline_configurations = build_baseline_configuration_runners(&args.baseline_config_paths)
        .await
        .context("Failed to build baseline node configurations")?;

    info!(
        "Running with the following configuration: {:#?}",
        baseline_configurations.0
    );

    let api = Api {
        baseline_configurations,
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
