// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, path::PathBuf};

use crate::{
    configuration::{read_configuration_from_file, NodeConfiguration},
    metric_collector::{MetricCollector, ReqwestMetricCollector},
    metric_evaluator::build_evaluators as build_metric_evaluators,
    runner::{BlockingRunner, Runner},
    system_information_evaluator::build_evaluators as build_system_information_evaluators,
};
use anyhow::{Context, Result};
use std::collections::HashSet;

/// This struct is a wrapper to help with all the different baseline
/// node configurations.
#[derive(Debug)]
pub struct ConfigurationsManager<M: MetricCollector, R: Runner> {
    /// The key here is the configuration_name.
    pub configurations: HashMap<String, NodeConfigurationWrapper<M, R>>,
}

#[derive(Debug)]
pub struct NodeConfigurationWrapper<M: MetricCollector, R: Runner> {
    pub node_configuration: NodeConfiguration,
    pub baseline_metric_collector: M,
    pub runner: R,
}

// In this function we finally build our trait objects with concrete implementations.
// We've piped trait bounds throughout our code but here we're finally facing the
// music and actually choosing some concrete types.
fn build_node_configuration_wrapper_with_blocking_runner_and_reqwest_metric_collector(
    node_configuration: NodeConfiguration,
) -> Result<NodeConfigurationWrapper<ReqwestMetricCollector, BlockingRunner<ReqwestMetricCollector>>>
{
    let baseline_metric_collector = ReqwestMetricCollector::new(
        node_configuration.node_address.url.clone(),
        node_configuration.node_address.metrics_port,
    );

    let mut evaluator_strings: HashSet<String> =
        node_configuration.evaluators.iter().cloned().collect();

    let metric_evaluators =
        build_metric_evaluators(&mut evaluator_strings, &node_configuration.evaluator_args)
            .context("Failed to build metric evaluators")?;

    let system_information_evaluators = build_system_information_evaluators(
        &mut evaluator_strings,
        &node_configuration.evaluator_args,
    )
    .context("Failed to build system information evaluators")?;

    let runner = BlockingRunner::new(
        node_configuration.runner_args.blocking_runner_args.clone(),
        baseline_metric_collector.clone(),
        metric_evaluators,
        system_information_evaluators,
    );

    let wrapper = NodeConfigurationWrapper {
        node_configuration,
        // TODO: Consider just fetching this from the runner instead.
        baseline_metric_collector,
        runner,
    };

    Ok(wrapper)
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
