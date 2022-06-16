// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, convert::TryFrom, path::PathBuf, sync::Arc};

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
use poem::{
    handler, http::StatusCode, listener::TcpListener, Error as PoemError, Result as PoemResult,
    Route, Server,
};
use poem_openapi::{payload::Json, OpenApi, OpenApiService};
use reqwest::blocking;
use url::Url;

/// This struct is a wrapper to help with all the different baseline
/// node configurations.
#[derive(Debug)]
pub struct ConfigurationsManager<M: MetricCollector, R: Runner> {
    /// They key here is the configuration_name.
    pub configurations: HashMap<String, NodeConfigurationWrapper<M, R>>,
}

#[derive(Debug)]
pub struct NodeConfigurationWrapper<M: MetricCollector, R: Runner> {
    pub node_configuration: NodeConfiguration,
    pub baseline_metric_collector: M,
    pub runner: R,
}

// Gross function name but there isn't a great way to do this programmatically.
// We've piped trait bounds throughout our code but here we're finally facing
// the music and actually choosing some concrete types).
fn build_node_configuration_wrapper_with_blocking_runner_and_reqwest_metric_collector(
    node_configuration: NodeConfiguration,
) -> Result<NodeConfigurationWrapper<ReqwestMetricCollector, BlockingRunner<ReqwestMetricCollector>>>
{
    let baseline_metric_collector = ReqwestMetricCollector::new(
        node_configuration.node_address.url.clone(),
        node_configuration.node_address.metrics_port,
    );

    let evaluators = build_evaluators(
        &node_configuration.evaluators,
        &node_configuration.evaluator_args,
    )
    .context("Failed to build evaluators")?;

    let runner = BlockingRunner::new(
        node_configuration.runner_args.blocking_runner_args.clone(),
        baseline_metric_collector.clone(),
        evaluators,
    );

    let w = NodeConfigurationWrapper {
        node_configuration,
        // TODO: Consider just fetching this from the runner instead.
        baseline_metric_collector,
        runner,
    };

    Ok(w)
}

pub async fn build_server_with_blocking_runner_and_reqwest_metric_collector(
    baseline_node_config_paths: &[PathBuf],
) -> Result<ConfigurationsManager<ReqwestMetricCollector, BlockingRunner<ReqwestMetricCollector>>> {
    let mut configurations = HashMap::new();
    for path in baseline_node_config_paths.iter() {
        let mut cfg = read_configuration_from_file(path.to_path_buf())
            .with_context(|| format!("Failed to read configuration from {}", path.display()))?;
        let name = cfg.configuration_name.clone();

        cfg.fetch_additional_configuration()
            .await
            .with_context(|| format!("Failed to fetch chain ID and role type for {}", name))?;

        let configuration_wrapper =
            build_node_configuration_wrapper_with_blocking_runner_and_reqwest_metric_collector(
                cfg,
            )?;
        configurations.insert(name, configuration_wrapper);
    }
    Ok(ConfigurationsManager { configurations })
}
