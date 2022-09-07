// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, path::PathBuf};

use crate::{
    configuration::{read_configuration_from_file, NodeConfiguration},
    evaluator::Evaluator,
    evaluators::{build_evaluators, direct::NodeIdentityEvaluator},
    metric_collector::ReqwestMetricCollector,
    runner::{BlockingRunner, Runner},
};
use anyhow::{Context, Result};

use super::NodeInformation;

/// This struct is a wrapper to help with all the different baseline
/// node configurations.
#[derive(Debug)]
pub struct ConfigurationsManager<R: Runner> {
    /// The key here is the configuration_name.
    pub configurations: HashMap<String, NodeConfigurationWrapper<R>>,
}

#[derive(Debug)]
pub struct NodeConfigurationWrapper<R: Runner> {
    pub node_configuration: NodeConfiguration,
    pub runner: R,
}

// In this function we finally build our trait objects with concrete implementations.
// We've piped trait bounds throughout our code but here we're finally facing the
// music and actually choosing some concrete types.
fn build_node_configuration_wrapper_with_blocking_runner(
    node_configuration: NodeConfiguration,
) -> Result<NodeConfigurationWrapper<BlockingRunner<ReqwestMetricCollector>>> {
    let baseline_node_information = NodeInformation {
        node_address: node_configuration.node_address.clone(),
        chain_id: node_configuration.get_chain_id(),
        role_type: node_configuration.get_role_type(),
    };

    let baseline_metric_collector =
        ReqwestMetricCollector::new(node_configuration.node_address.clone());

    let evaluator_set = build_evaluators(
        &node_configuration.evaluators,
        &node_configuration.evaluator_args,
    )
    .context("Failed to build evaluators")?;

    let node_identity_evaluator =
        NodeIdentityEvaluator::from_evaluator_args(&node_configuration.evaluator_args)?;

    let runner = BlockingRunner::new(
        node_configuration.runner_args.blocking_runner_args.clone(),
        baseline_node_information,
        baseline_metric_collector,
        node_identity_evaluator,
        evaluator_set,
    );

    let wrapper = NodeConfigurationWrapper {
        node_configuration,
        runner,
    };

    Ok(wrapper)
}

pub async fn build_server_with_blocking_runner(
    baseline_node_config_paths: &[PathBuf],
) -> Result<ConfigurationsManager<BlockingRunner<ReqwestMetricCollector>>> {
    let mut configurations = HashMap::new();
    for path in baseline_node_config_paths.iter() {
        let mut cfg = read_configuration_from_file(path.to_path_buf())
            .with_context(|| format!("Failed to read configuration from {}", path.display()))?;
        let name = cfg.configuration_name.clone();

        cfg.fetch_additional_configuration()
            .await
            .with_context(|| format!("Failed to fetch chain ID and role type for {}", name))?;

        let configuration_wrapper = build_node_configuration_wrapper_with_blocking_runner(cfg)?;
        configurations.insert(name, configuration_wrapper);
    }
    Ok(ConfigurationsManager { configurations })
}
