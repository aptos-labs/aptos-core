// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::accept_type::AcceptType;
use crate::context::Context;
use crate::response::InternalError;
use crate::response::ServiceUnavailableError;
use crate::{generate_error_response, generate_success_response, ApiTags};
use anyhow::Context as AnyhowContext;
use aptos_api_types::AptosErrorCode;
use poem_openapi::{param::Query, payload::Html, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::ops::Sub;
use std::{
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
            message: "aptos-node:ok".to_string(),
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
        let ledger_info = self.context.get_latest_ledger_info()?;

        // If we have a duration, check that it's close to the current time, otherwise it's ok
        if let Some(duration) = duration_secs.0 {
            let timestamp = ledger_info.timestamp();

            let timestamp = Duration::from_micros(timestamp);
            let expectation = SystemTime::now()
                .sub(Duration::from_secs(duration as u64))
                .duration_since(UNIX_EPOCH)
                .context("Failed to determine absolute unix time based on given duration")
                .map_err(|err| {
                    HealthCheckError::internal_with_code(
                        err,
                        AptosErrorCode::InternalError,
                        &ledger_info,
                    )
                })?;

            if timestamp < expectation {
                return Err(HealthCheckError::service_unavailable_with_code(
                    "The latest ledger info timestamp is less than the expected timestamp",
                    AptosErrorCode::HealthCheckFailed,
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
