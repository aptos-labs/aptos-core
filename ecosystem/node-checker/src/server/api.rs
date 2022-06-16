// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::NodeAddress,
    metric_collector::{MetricCollector, ReqwestMetricCollector},
    metric_evaluator::EvaluationSummary,
    runner::Runner,
};
use anyhow::anyhow;
use poem::{http::StatusCode, Error as PoemError, Result as PoemResult};
use poem_openapi::{payload::Json, types::Example, Object as PoemObject, OpenApi, OpenApiService};
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
// make for more descriptive error messages.
#[OpenApi]
impl<M: MetricCollector, R: Runner> Api<M, R> {
    /// todo
    #[oai(path = "/check_node", method = "get")]
    async fn check_node(
        &self,
        request: Json<CheckNodeRequest>,
    ) -> PoemResult<Json<EvaluationSummary>> {
        if self.allow_preconfigured_test_node_only {
            return Err(PoemError::from((
                StatusCode::METHOD_NOT_ALLOWED,
                anyhow!(
                "This node health checker is configured to only check its preconfigured test node"),
            )));
        }
        // todo check if this is necessary now that the type is URL, and if it is, use set_scheme instead.
        let mut target_url = request.target_node.url.to_string();
        if !target_url.starts_with("http") {
            target_url = format!("http://{}", target_url);
        }
        let target_url = match Url::parse(&target_url) {
            Ok(url) => url,
            Err(e) => return Err(PoemError::from((StatusCode::BAD_REQUEST, anyhow!(e)))),
        };

        let baseline_node_configuration =
            self.get_baseline_node_configuration(&request.baseline_configuration_name)?;

        let target_metric_collector =
            ReqwestMetricCollector::new(target_url, request.target_node.metrics_port);

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

    /// todo
    #[oai(path = "/check_preconfigured_node", method = "get")]
    async fn check_preconfigured_node(
        &self,
        baseline_configuration_name: Json<Option<String>>,
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
    endpoint_override: Option<&str>,
) -> OpenApiService<Api<M, R>, ()> {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    // TODO: Ensure we have a scheme on listen address.
    let host = listen_address
        .host_str()
        .expect("Failed to find host in listen address");
    let endpoint = endpoint_override.unwrap_or("api");
    let api_service = OpenApiService::new(api, "Aptos Node Checker", version).server(format!(
        "{}://{}:{}/{}",
        listen_address.scheme(),
        host,
        listen_port,
        endpoint
    ));
    api_service
}
