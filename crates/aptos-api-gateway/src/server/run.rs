// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::server_config::ServerConfig;
use crate::{
    bypasser::BypasserConfig,
    checkers::CheckerConfig,
    grpc_proxy::{grpc_proxy::GrpcProxy, grpc_proxy_config::GrpcProxyConfig},
};
use anyhow::{Context, Result};
use aptos_faucet_metrics_server::{run_metrics_server, MetricsServerConfig};
use aptos_logger::info;
use clap::Parser;
use poem::{listener::TcpListener, Server};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::BufReader, path::PathBuf, pin::Pin};
use tokio::task::JoinSet;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct HandlerConfig {
    /// Whether we should return helpful errors.
    pub use_helpful_errors: bool,

    /// Whether we should return rejections the moment a Checker returns any,
    /// or should instead run through all Checkers first. Generally prefer
    /// setting this to true, as it is less work on the tap, but setting it
    /// to false does give the user more immediate information.
    pub return_rejections_early: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunConfig {
    /// API server config.
    pub server_config: ServerConfig,

    /// Metrics server config.
    pub metrics_server_config: MetricsServerConfig,

    /// Configs for any Bypassers we might want to enable.
    pub bypasser_configs: Vec<BypasserConfig>,

    /// Configs for any Checkers we might want to enable.
    pub checker_configs: Vec<CheckerConfig>,

    /// General args for the runner / handler.
    pub handler_config: HandlerConfig,

    pub grpc_proxy_config: GrpcProxyConfig,
}

impl RunConfig {
    pub async fn run(self) -> Result<()> {
        info!("Running with config: {:#?}", self);
        info!("Starting api-gateway...");

        // Create a periodic task manager.
        let mut join_set = JoinSet::new();

        // Collect futures that should never end.
        let mut main_futures: Vec<Pin<Box<dyn futures::Future<Output = Result<()>> + Send>>> =
            Vec::new();

        // Create a future for the metrics server.
        if !self.metrics_server_config.disable {
            main_futures.push(Box::pin(async move {
                run_metrics_server(self.metrics_server_config.clone())
                    .await
                    .context("Metrics server ended unexpectedly")
            }));
        }

        let grpc_proxy = GrpcProxy::new(self.grpc_proxy_config.upstream_host.clone());

        // Create a future for the API server.
        let api_server_future = Server::new(TcpListener::bind((
            self.server_config.listen_address.clone(),
            self.server_config.listen_port,
        )))
        .run(
            grpc_proxy,
            // Route::new()
            //     .at(grpc_proxy)
            //     // .nest(
            //     //     &self.server_config.api_path_base,
            //     //     Route::new()
            //     //         .nest("", api_service)
            //     //         .catch_all_error(convert_error),
            //     // )
            //     // .at("/spec.json", spec_json)
            //     // .at("/spec.yaml", spec_yaml)
            //     // .with(cors)
            //     .around(middleware_log),
        );

        main_futures.push(Box::pin(async move {
            api_server_future
                .await
                .context("API server ended unexpectedly")
        }));

        // If there are any periodic tasks, create a future for retrieving
        // one so we know if any of them unexpectedly end.
        if !join_set.is_empty() {
            main_futures.push(Box::pin(async move {
                join_set.join_next().await.unwrap().unwrap()
            }));
        }

        println!(
            "Faucet is running. Faucet endpoint: http://{}:{}",
            self.server_config.listen_address, self.server_config.listen_port
        );

        // Wait for all the futures. We expect none of them to ever end.
        futures::future::select_all(main_futures)
            .await
            .0
            .context("One of the futures that were not meant to end ended unexpectedly")
    }

    /// Like `run` but manipulates the server config for a test environment.
    pub async fn run_test(mut self, port: u16) -> Result<()> {
        self.server_config.listen_port = port;
        self.metrics_server_config.disable = true;
        self.run().await
    }
}

#[derive(Clone, Debug, Parser)]
pub struct Run {
    #[clap(short, long, parse(from_os_str))]
    config_path: PathBuf,
}

impl Run {
    pub async fn run(&self) -> Result<()> {
        let run_config = self.get_run_config()?;
        run_config.run().await
    }

    pub fn get_run_config(&self) -> Result<RunConfig> {
        let file = File::open(&self.config_path).with_context(|| {
            format!(
                "Failed to load config at {}",
                self.config_path.to_string_lossy()
            )
        })?;
        let reader = BufReader::new(file);
        let run_config: RunConfig = serde_yaml::from_reader(reader).with_context(|| {
            format!(
                "Failed to parse config at {}",
                self.config_path.to_string_lossy()
            )
        })?;
        Ok(run_config)
    }
}
