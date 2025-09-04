// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
use super::{build::BaselineConfigurationRunners, common::ServerArgs};
use crate::{configuration::NodeAddress, runner::Runner, CheckSummary};
use anyhow::{anyhow, Context};
use velor_crypto::{x25519, ValidCryptoMaterialStringExt};
use velor_logger::error;
use poem::http::StatusCode;
use poem_openapi::{param::Query, payload::Json, Object, OpenApi, OpenApiService};
use std::convert::TryInto;
use url::Url;

pub struct Api<R: Runner> {
    pub baseline_configurations: BaselineConfigurationRunners<R>,
}

// I choose to keep both methods rather than making these two separate APIs because it'll
// make for more descriptive error messages. We write the function comment on one line
// because the OpenAPI generator does some wonky newline stuff otherwise. Currently Poem
// doesn't support "flattening" a struct into separate query parameters, so I do that
// myself. See https://github.com/poem-web/poem/issues/241.
#[OpenApi]
impl<R: Runner> Api<R> {
    /// Check the health of a given target node. You must specify a baseline
    /// node configuration to use for the evaluation and the URL of your node,
    /// without including any port or endpoints. All other parameters are optional.
    /// For example, if your node's API port is open but the rest are closed, only
    /// set the `api_port`.
    #[oai(path = "/check", method = "get")]
    async fn check(
        &self,
        /// The ID of the baseline node configuration to use for the evaluation, e.g. devnet_fullnode
        baseline_configuration_id: Query<String>,
        /// The URL of the node to check, e.g. http://44.238.19.217 or http://fullnode.mysite.com
        node_url: Query<Url>,
        /// If given, we will assume the metrics service is available at the given port.
        metrics_port: Query<Option<u16>>,
        /// If given, we will assume the API is available at the given port.
        api_port: Query<Option<u16>>,
        /// If given, we will assume that clients can communicate with your node via noise at the given port.
        noise_port: Query<Option<u16>>,
        /// A public key for the node, e.g. 0x44fd1324c66371b4788af0b901c9eb8088781acb29e6b8b9c791d5d9838fbe1f.
        /// This is only necessary for certain checkers, e.g. HandshakeChecker.
        public_key: Query<Option<String>>,
    ) -> poem::Result<Json<CheckSummary>> {
        // Ensure the public key, if given, is in a valid format.
        let public_key = match public_key.0 {
            Some(public_key) => match x25519::PublicKey::from_encoded_string(&public_key) {
                Ok(public_key) => Some(public_key),
                Err(e) => {
                    return Err(poem::Error::from((
                        StatusCode::BAD_REQUEST,
                        anyhow!("Invalid public key \"{}\": {:#}", public_key, e),
                    )))
                },
            },
            None => None,
        };

        let baseline_configuration = self
            .baseline_configurations
            .0
            .get(&baseline_configuration_id.0)
            .context(format!(
                "Baseline configuration {} does not exist",
                baseline_configuration_id.0
            ))
            .map_err(|e| poem::Error::from((StatusCode::BAD_REQUEST, e)))?;

        // Within a single NHC run we want to use the same client so that cookies
        // can be collected and used. This is important because the nodes we're
        // talking to might be a behind a LB that does cookie based sticky routing.
        // If we don't do this, we can get read inconsistency, e.g. where we read
        // that the node has transaction version X, but then we fail to retrieve the
        // transaction at the version because the LB routes us to a different node.
        // In this function, which comprises a single NHC run, we build a NodeAddress
        // for the target and use that throughout the request. Further functions
        // deeper down might clone these structs, but that is fine, because the
        // important part, the CookieStore (Jar) is in an Arc, so each time we clone
        // the struct we're just cloning the reference to the same jar.
        let target_node_address = NodeAddress::new(
            node_url.0,
            api_port.0,
            metrics_port.0,
            noise_port.0,
            public_key,
        );

        let complete_evaluation_result = baseline_configuration
            .runner
            .run(&target_node_address)
            .await;

        match complete_evaluation_result {
            Ok(complete_evaluation) => Ok(Json(complete_evaluation)),
            Err(e) => {
                // We only get to this point if the evaluation failed due to an error
                // on our side, e.g. something wrong with NHC or the baseline.
                error!(
                    target_node_url = target_node_address.url,
                    event = "check_failed_our_fault"
                );
                Err(poem::Error::from((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    anyhow!(e),
                )))
            },
        }
    }

    /// Get the IDs and pretty names for the configurations. For example,
    /// devnet_fullnode as the ID and "Devnet Fullnode Checker" as the
    /// pretty name.
    #[oai(path = "/configurations", method = "get")]
    async fn configurations(&self) -> Json<Vec<ConfigurationDescriptor>> {
        Json(
            self.baseline_configurations
                .0
                .iter()
                .map(|(k, v)| ConfigurationDescriptor {
                    id: k.clone(),
                    pretty_name: v.configuration.configuration_name.clone(),
                })
                .collect(),
        )
    }
}

#[derive(Clone, Debug, Object)]
struct ConfigurationDescriptor {
    /// Configuration ID, for example devnet_fullnode.
    pub id: String,
    /// Configuration pretty name, for example "Devnet Fullnode Checker".
    pub pretty_name: String,
}

pub fn build_openapi_service<R: Runner>(
    api: Api<R>,
    server_args: ServerArgs,
) -> OpenApiService<Api<R>, ()> {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.2.0".to_string());
    // These should have already been validated at this point, so we panic.
    let url: Url = server_args
        .try_into()
        .expect("Failed to parse listen address");
    OpenApiService::new(api, "Velor Node Checker", version).server(url)
}
