// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    context::{api_spawn_blocking, Context},
    generate_error_response, generate_success_response,
    response::{InternalError, ServiceUnavailableError},
    ApiTags,
};
use anyhow::Context as AnyhowContext;
use velor_api_types::VelorErrorCode;
use poem_openapi::{
    param::Query,
    payload::{Html, Json},
    Object, OpenApi,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ops::Sub,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const OPEN_API_HTML: &str = include_str!("../doc/spec.html");

// Generate error and response types
generate_success_response!(HealthCheckResponse, (200, Ok));
generate_error_response!(HealthCheckError, (503, ServiceUnavailable), (500, Internal));
pub type HealthCheckResult<T> = poem::Result<HealthCheckResponse<T>, HealthCheckError>;

/// Basic API does healthchecking and shows the OpenAPI spec
pub struct BasicApi {
    pub context: Arc<Context>,
}

/// Representation of a successful healthcheck
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, Object)]
pub struct HealthCheckSuccess {
    message: String,
}

impl HealthCheckSuccess {
    pub fn new() -> Self {
        Self {
            message: "velor-node:ok".to_string(),
        }
    }
}

#[OpenApi]
impl BasicApi {
    /// Show OpenAPI explorer
    ///
    /// Provides a UI that you can use to explore the API. You can also
    /// retrieve the API directly at `/spec.yaml` and `/spec.json`.
    #[oai(
        path = "/spec",
        method = "get",
        operation_id = "spec",
        tag = "ApiTags::General"
    )]
    async fn spec(&self) -> Html<String> {
        Html(OPEN_API_HTML.to_string())
    }

    /// Show some basic info of the node.
    #[oai(
        path = "/info",
        method = "get",
        operation_id = "info",
        tag = "ApiTags::General"
    )]
    async fn info(&self) -> Json<HashMap<String, serde_json::Value>> {
        let mut info = HashMap::new();

        // Insert state sync configuration information
        info.insert(
            "bootstrapping_mode".to_string(),
            serde_json::to_value(
                self.context
                    .node_config
                    .state_sync
                    .state_sync_driver
                    .bootstrapping_mode,
            )
            .unwrap(),
        );
        info.insert(
            "continuous_syncing_mode".to_string(),
            serde_json::to_value(
                self.context
                    .node_config
                    .state_sync
                    .state_sync_driver
                    .continuous_syncing_mode,
            )
            .unwrap(),
        );

        // Insert storage configuration information
        info.insert(
            "new_storage_format".to_string(),
            serde_json::to_value(
                self.context
                    .node_config
                    .storage
                    .rocksdb_configs
                    .enable_storage_sharding,
            )
            .unwrap(),
        );
        info.insert(
            "internal_indexer_config".to_string(),
            serde_json::to_value(&self.context.node_config.indexer_db_config).unwrap(),
        );

        // Insert node identity information
        if let Some(validator_network) = &self.context.node_config.validator_network {
            info.insert(
                "validator_network_peer_id".to_string(),
                serde_json::to_value(validator_network.peer_id()).unwrap(),
            );
        }
        for fullnode_network in &self.context.node_config.full_node_networks {
            info.insert(
                format!("fullnode_network_peer_id_{}", fullnode_network.network_id),
                serde_json::to_value(fullnode_network.peer_id()).unwrap(),
            );
        }

        Json(info)
    }

    /// Check basic node health
    ///
    /// By default this endpoint just checks that it can get the latest ledger
    /// info and then returns 200.
    ///
    /// If the duration_secs param is provided, this endpoint will return a
    /// 200 if the following condition is true:
    ///
    /// `server_latest_ledger_info_timestamp >= server_current_time_timestamp - duration_secs`
    #[oai(
        path = "/-/healthy",
        method = "get",
        operation_id = "healthy",
        tag = "ApiTags::General"
    )]
    async fn healthy(
        &self,
        accept_type: AcceptType,
        /// Threshold in seconds that the server can be behind to be considered healthy
        ///
        /// If not provided, the healthcheck will always succeed
        duration_secs: Query<Option<u32>>,
    ) -> HealthCheckResult<HealthCheckSuccess> {
        let context = self.context.clone();
        let ledger_info = api_spawn_blocking(move || context.get_latest_ledger_info()).await?;

        // If we have a duration, check that it's close to the current time, otherwise it's ok
        if let Some(max_skew) = duration_secs.0 {
            let ledger_timestamp = Duration::from_micros(ledger_info.timestamp());
            let skew_threshold = SystemTime::now()
                .sub(Duration::from_secs(max_skew as u64))
                .duration_since(UNIX_EPOCH)
                .context("Failed to determine absolute unix time based on given duration")
                .map_err(|err| {
                    HealthCheckError::internal_with_code(
                        err,
                        VelorErrorCode::InternalError,
                        &ledger_info,
                    )
                })?;

            if ledger_timestamp < skew_threshold {
                return Err(HealthCheckError::service_unavailable_with_code(
                    format!("The latest ledger info timestamp is {:?}, which is beyond the allowed skew ({}s).", ledger_timestamp, max_skew),
                    VelorErrorCode::HealthCheckFailed,
                    &ledger_info,
                ));
            }
        }
        HealthCheckResponse::try_from_rust_value((
            HealthCheckSuccess::new(),
            &ledger_info,
            HealthCheckResponseStatus::Ok,
            &accept_type,
        ))
    }
}
