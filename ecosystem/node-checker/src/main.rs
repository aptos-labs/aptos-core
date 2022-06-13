// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// For use while we're developing.
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod args;
mod build_evaluators;
mod metric_collector;
mod metric_evaluator;
mod public_types;
mod runner;

use anyhow::{anyhow, bail, Context, Result};
use args::Args;
use build_evaluators::build_evaluators;
use clap::Parser;
use log::{debug, info};
use metric_collector::{MetricCollector, ReqwestMetricCollector};
use metric_evaluator::MetricsEvaluator;
use poem::{
    handler, http::StatusCode, listener::TcpListener, Error as PoemError, Result as PoemResult,
    Route, Server,
};
use poem_openapi::{payload::Json, OpenApi, OpenApiService};
use public_types::{EvaluationSummary, NodeUrl};
use reqwest::Client as ReqwestClient;
use runner::{BlockingRunner, BlockingRunnerArgs, Runner};
use std::{collections::HashSet, hash::Hash, path::PathBuf, sync::Arc};
use url::Url;

// TODO: Replace this with the real frontend, or perhaps an error handler if we
// decide to route the frontend to just a static hoster such as nginx.
#[handler]
fn root() -> String {
    "TODO: Under construction!".to_string()
}

struct Api<M: MetricCollector, R: Runner> {
    pub runner: R,
    pub target_metric_collector: Option<M>,
    pub allow_preconfigured_node_only: bool,
}

// I choose to keep both methods rather than making these two separate APIs because it'll
// make for more descriptive error messages.
#[OpenApi]
impl<M: MetricCollector, R: Runner> Api<M, R> {
    #[oai(path = "/check_node", method = "get")]
    async fn check_node(&self, target_node: Json<NodeUrl>) -> PoemResult<Json<EvaluationSummary>> {
        if self.allow_preconfigured_node_only {
            return Err(PoemError::from((
                StatusCode::METHOD_NOT_ALLOWED,
                anyhow!(
                "This node health checker is configured to only check its preconfigured test node"),
            )));
        }
        let mut target_url = target_node.url.to_string();
        if !target_url.starts_with("http") {
            target_url = format!("http://{}", target_url);
        }
        let target_url = match Url::parse(&target_url) {
            Ok(url) => url,
            Err(e) => return Err(PoemError::from((StatusCode::BAD_REQUEST, anyhow!(e)))),
        };
        let target_metric_collector =
            ReqwestMetricCollector::new(target_url, target_node.metrics_port);
        let complete_evaluation_result = self.runner.run(&target_metric_collector).await;
        match complete_evaluation_result {
            Ok(complete_evaluation) => Ok(Json(complete_evaluation)),
            Err(e) => Err(PoemError::from((
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow!(e),
            ))),
        }
    }

    #[oai(path = "/check_preconfigured_node", method = "get")]
    async fn check_preconfigured_node(&self) -> PoemResult<Json<EvaluationSummary>> {
        if self.target_metric_collector.is_none() {
            return Err(PoemError::from((
                StatusCode::METHOD_NOT_ALLOWED,
                anyhow!(
                    "This node health checker has not been set up with a preconfigured test node"
                ),
            )));
        }
        let complete_evaluation_result = self
            .runner
            .run(self.target_metric_collector.as_ref().unwrap())
            .await;
        match complete_evaluation_result {
            Ok(complete_evaluation) => Ok(Json(complete_evaluation)),
            // Consider returning error codes within the response.
            Err(e) => Err(PoemError::from((
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow!(e),
            ))),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());

    let baseline_metric_collector =
        ReqwestMetricCollector::new(args.baseline_node_url.clone(), args.baseline_metrics_port);

    let target_metric_collector = match args.target_node_url {
        Some(ref url) => Some(ReqwestMetricCollector::new(
            url.clone(),
            args.target_metrics_port,
        )),
        None => None,
    };

    let evaluators = build_evaluators(&args).context("Failed to build evaluators")?;

    let runner = BlockingRunner::new(
        args.blocking_runner_args,
        baseline_metric_collector,
        evaluators,
    );

    let api = Api {
        runner,
        target_metric_collector,
        allow_preconfigured_node_only: args.allow_preconfigured_node_only,
    };

    let api_service = OpenApiService::new(api, "Aptos Node Checker", version).server(format!(
        "http://{}:{}/api",
        args.listen_address, args.listen_port
    ));
    let ui = api_service.swagger_ui();
    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    Server::new(TcpListener::bind((args.listen_address, args.listen_port)))
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
