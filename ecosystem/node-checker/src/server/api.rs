// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::{NodeAddress, NodeConfiguration},
    evaluator::EvaluationSummary,
    metric_collector::{MetricCollector, ReqwestMetricCollector},
    runner::Runner,
};
use anyhow::anyhow;
use poem::{http::StatusCode, Error as PoemError, Result as PoemResult};
use poem_openapi::{
    param::Query, payload::Json, types::Example, Object as PoemObject, OpenApi, OpenApiService,
};
use url::Url;

use super::configurations_manager::{ConfigurationsManager, NodeConfigurationWrapper};

pub struct Api<M: MetricCollector, R: Runner> {
    pub configurations_manager: ConfigurationsManager<M, R>,
    pub target_metric_collector: Option<M>,
    pub allow_preconfigured_test_node_only: bool,
}

impl<M: MetricCollector, R: Runner> Api<M, R> {
    fn get_baseline_node_configuration(
        &self,
        baseline_configuration_name: &Option<String>,
    ) -> PoemResult<&NodeConfigurationWrapper<M, R>> {
        let baseline_configuration_name = match baseline_configuration_name {
            Some(name) => name,
            // TODO: Auto detect this based on the target node.
            None => {
                return Err(PoemError::from((
                    StatusCode::BAD_REQUEST,
                    anyhow!("You must provide a baseline configuration name for now"),
                )))
            }
        };
        let node_configuration = match self
            .configurations_manager
            .configurations
            .get(baseline_configuration_name)
        {
            Some(runner) => runner,
            None => {
                return Err(PoemError::from((
                    StatusCode::BAD_REQUEST,
                    anyhow!(
                        "No baseline configuration found with name {}",
                        baseline_configuration_name
                    ),
                )))
            }
        };
        Ok(node_configuration)
    }
}

// I choose to keep both methods rather than making these two separate APIs because it'll
// make for more descriptive error messages. We write the function comment on one line
// because the OpenAPI generator does some wonky newline stuff otherwise. Currently Poem
// doesn't support "flattening" a struct into separate query parameters, so I do that
// myself. See https://github.com/poem-web/poem/issues/241.
#[OpenApi]
impl<M: MetricCollector, R: Runner> Api<M, R> {
    /// Check the health of a given target node. You may specify a baseline node configuration to use for the evaluation. If you don't specify a baseline node configuration, we will attempt to determine the appropriate baseline based on your target node.
    #[oai(path = "/check_node", method = "get")]
    async fn check_node(
        &self,
        /// The URL of the node to check. e.g. http://44.238.19.217 or http://fullnode.mysite.com
        node_url: Query<Url>,
        /// The name of the baseline node configuration to use for the evaluation, e.g. devnet_fullnode
        baseline_configuration_name: Query<Option<String>>,
        #[oai(default = "NodeAddress::default_metrics_port")] metrics_port: Query<u16>,
        #[oai(default = "NodeAddress::default_api_port")] api_port: Query<u16>,
        #[oai(default = "NodeAddress::default_noise_port")] noise_port: Query<u16>,
    ) -> PoemResult<Json<EvaluationSummary>> {
        let request = CheckNodeRequest {
            baseline_configuration_name: baseline_configuration_name.0,
            target_node: NodeAddress {
                url: node_url.0,
                metrics_port: metrics_port.0,
                api_port: api_port.0,
                noise_port: noise_port.0,
            },
        };
        if self.allow_preconfigured_test_node_only {
            return Err(PoemError::from((
                StatusCode::METHOD_NOT_ALLOWED,
                anyhow!(
                "This node health checker is configured to only check its preconfigured test node"),
            )));
        }

        let baseline_node_configuration =
            self.get_baseline_node_configuration(&request.baseline_configuration_name)?;

        let target_metric_collector = ReqwestMetricCollector::new(
            request.target_node.url.clone(),
            request.target_node.metrics_port,
        );

        let complete_evaluation_result = baseline_node_configuration
            .runner
            .run(&target_metric_collector)
            .await;

        match complete_evaluation_result {
            Ok(complete_evaluation) => Ok(Json(complete_evaluation)),
            Err(e) => Err(PoemError::from((
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow!(e),
            ))),
        }
    }

    /// Check the health of the preconfigured node. If none was specified when this instance of the node checker was started, this will return an error. You may specify a baseline node configuration to use for the evaluation. If you don't specify a baseline node configuration, we will attempt to determine the appropriate baseline based on your target node.
    #[oai(path = "/check_preconfigured_node", method = "get")]
    async fn check_preconfigured_node(
        &self,
        baseline_configuration_name: Query<Option<String>>,
    ) -> PoemResult<Json<EvaluationSummary>> {
        if self.target_metric_collector.is_none() {
            return Err(PoemError::from((
                StatusCode::METHOD_NOT_ALLOWED,
                anyhow!(
                    "This node health checker has not been set up with a preconfigured test node"
                ),
            )));
        }

        let baseline_node_configuration =
            self.get_baseline_node_configuration(&baseline_configuration_name)?;

        let complete_evaluation_result = baseline_node_configuration
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

    #[oai(path = "/get_configurations", method = "get")]
    async fn get_configurations(&self) -> Json<Vec<NodeConfiguration>> {
        Json(
            self.configurations_manager
                .configurations
                .values()
                .map(|n| n.node_configuration.clone())
                .collect(),
        )
    }

    #[oai(path = "/get_configuration_keys", method = "get")]
    async fn get_configuration_keys(&self) -> Json<Vec<String>> {
        Json(
            self.configurations_manager
                .configurations
                .keys()
                .cloned()
                .collect(),
        )
    }
}

#[derive(Clone, Debug, PoemObject)]
#[oai(example)]
struct CheckNodeRequest {
    target_node: NodeAddress,
    baseline_configuration_name: Option<String>,
}

impl Example for CheckNodeRequest {
    fn example() -> Self {
        Self {
            baseline_configuration_name: Some("Devnet Full Node".to_string()),
            target_node: NodeAddress::example(),
        }
    }
}

pub fn build_openapi_service<M: MetricCollector, R: Runner>(
    api: Api<M, R>,
    listen_address: Url,
    listen_port: u16,
    endpoint: &str,
) -> OpenApiService<Api<M, R>, ()> {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    // TODO: Ensure we have a scheme on listen address.
    let host = listen_address
        .host_str()
        .expect("Failed to find host in listen address");
    let api_service = OpenApiService::new(api, "Aptos Node Checker", version).server(format!(
        "{}://{}:{}/{}",
        listen_address.scheme(),
        host,
        listen_port,
        endpoint
    ));
    api_service
}
