// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryInto;

use super::{
    common::ServerArgs,
    configurations_manager::{ConfigurationsManager, NodeConfigurationWrapper},
};
use crate::{
    configuration::{NodeAddress, NodeConfiguration},
    evaluator::EvaluationSummary,
    metric_collector::{MetricCollector, ReqwestMetricCollector},
    runner::Runner,
};
use anyhow::anyhow;
use aptos_crypto::x25519;
use aptos_crypto::ValidCryptoMaterialStringExt;
use poem::{http::StatusCode, Error as PoemError, Result as PoemResult};
use poem_openapi::{param::Query, payload::Json, Object as PoemObject, OpenApi, OpenApiService};
use url::Url;

pub struct PreconfiguredNode<M: MetricCollector> {
    pub node_address: NodeAddress,
    pub metric_collector: M,
}

pub struct Api<M: MetricCollector, R: Runner> {
    pub configurations_manager: ConfigurationsManager<R>,
    pub preconfigured_test_node: Option<PreconfiguredNode<M>>,
    pub allow_preconfigured_test_node_only: bool,
}

impl<M: MetricCollector, R: Runner> Api<M, R> {
    fn get_baseline_node_configuration(
        &self,
        baseline_configuration_name: &Option<String>,
    ) -> PoemResult<&NodeConfigurationWrapper<R>> {
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
    /// Check the health of a given target node. You may specify a baseline
    /// node configuration to use for the evaluation. If you don't specify
    /// a baseline node configuration, we will attempt to determine the
    /// appropriate baseline based on your target node.
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
        /// A public key for the node, e.g. 0x44fd1324c66371b4788af0b901c9eb8088781acb29e6b8b9c791d5d9838fbe1f.
        /// This is only necessary for certain evaluators, e.g. HandshakeEvaluator.
        public_key: Query<Option<String>>,
    ) -> PoemResult<Json<EvaluationSummary>> {
        // Ensure the public key, if given, is in a valid format.
        let public_key = match public_key.0 {
            Some(public_key) => match x25519::PublicKey::from_encoded_string(&public_key) {
                Ok(public_key) => Some(public_key),
                Err(e) => {
                    return Err(PoemError::from((
                        StatusCode::BAD_REQUEST,
                        anyhow!("Invalid public key \"{}\": {:#}", public_key, e),
                    )))
                }
            },
            None => None,
        };

        // Within a single NHC run we want to use the same client so that cookies
        // can be collected and used. This is important because the nodes we're
        // talking to might be a behind a LB that does cookie based sticky routing.
        // If we don't do this, we can get read inconsistency, e.g. where we read
        // that the node has transaction version X, but then we fail to retrieve the
        // transaction at the version because the LB routes us to a different node.
        // In this function, which comprises a single NHC run, we build a NodeAddress
        // for the baseline and target and use that throughout the request. Further
        // functions deeper down might clone these structs, but that is fine, because
        // the important part, the CookieStore (Jar) is in an Arc, so each time we
        // clone the struct we're just cloning the reference to the same jar.

        let target_node_address = NodeAddress::new(node_url.0)
            .metrics_port(metrics_port.0)
            .api_port(api_port.0)
            .noise_port(noise_port.0)
            .public_key(public_key);

        let baseline_node_configuration =
            self.get_baseline_node_configuration(&baseline_configuration_name.0)?;

        // Ensure the given arguments are valid for the configured evaluators.
        for evaluator in &baseline_node_configuration
            .runner
            .get_evaluator_set()
            .evaluators
        {
            if let Err(e) = evaluator.validate_check_node_call(&target_node_address) {
                return Err(PoemError::from((
                    StatusCode::BAD_REQUEST,
                    anyhow!("Invalid request: {}", e),
                )));
            }
        }

        if self.allow_preconfigured_test_node_only {
            return Err(PoemError::from((
                StatusCode::METHOD_NOT_ALLOWED,
                anyhow!(
                "This node health checker is configured to only check its preconfigured test node"),
            )));
        }

        let target_metric_collector = ReqwestMetricCollector::new(target_node_address.clone());

        let complete_evaluation_result = baseline_node_configuration
            .runner
            .run(&target_node_address, &target_metric_collector)
            .await;

        match complete_evaluation_result {
            Ok(complete_evaluation) => Ok(Json(complete_evaluation)),
            Err(e) => Err(PoemError::from((
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow!(e),
            ))),
        }
    }

    /// Check the health of the preconfigured node. If none was specified when
    /// this instance of the node checker was started, this will return an error.
    /// You may specify a baseline node configuration to use for the evaluation.
    /// If you don't specify a baseline node configuration, we will attempt to
    /// determine the appropriate baseline based on your target node.
    #[oai(path = "/check_preconfigured_node", method = "get")]
    async fn check_preconfigured_node(
        &self,
        baseline_configuration_name: Query<Option<String>>,
    ) -> PoemResult<Json<EvaluationSummary>> {
        if self.preconfigured_test_node.is_none() {
            return Err(PoemError::from((
                StatusCode::METHOD_NOT_ALLOWED,
                anyhow!(
                    "This node health checker has not been set up with a preconfigured test node"
                ),
            )));
        }
        let preconfigured_test_node = self.preconfigured_test_node.as_ref().unwrap();

        let baseline_node_configuration =
            self.get_baseline_node_configuration(&baseline_configuration_name)?;

        let complete_evaluation_result = baseline_node_configuration
            .runner
            .run(
                &preconfigured_test_node.node_address,
                &preconfigured_test_node.metric_collector,
            )
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

    /// Get the different baseline configurations the instance of NHC is
    /// configured with. This method is best effort, it is infeasible to
    /// derive (or even represent) some fields of the spec via OpenAPI,
    /// so note that some fields will be missing from the response.
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

    /// Get just the keys and pretty names for the configurations, meaning
    /// the configuration_name and configuration_name_pretty fields.
    #[oai(path = "/get_configuration_keys", method = "get")]
    async fn get_configuration_keys(&self) -> Json<Vec<ConfigurationKey>> {
        Json(
            self.configurations_manager
                .configurations
                .values()
                .map(|n| ConfigurationKey {
                    key: n.node_configuration.configuration_name.clone(),
                    pretty_name: n.node_configuration.configuration_name_pretty.clone(),
                })
                .collect(),
        )
    }
}

#[derive(Clone, Debug, PoemObject)]
struct ConfigurationKey {
    pub key: String,
    pub pretty_name: String,
}

pub fn build_openapi_service<M: MetricCollector, R: Runner>(
    api: Api<M, R>,
    server_args: ServerArgs,
) -> OpenApiService<Api<M, R>, ()> {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    // These should have already been validated at this point, so we panic.
    let url: Url = server_args
        .try_into()
        .expect("Failed to parse listen address");
    OpenApiService::new(api, "Aptos Node Checker", version).server(url)
}
