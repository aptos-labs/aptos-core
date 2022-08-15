// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::accept_type::AcceptType;
use crate::context::Context;
use crate::response::{BasicError, BasicResponse, BasicResponseStatus, BasicResult, InternalError};
use crate::ApiTags;
use anyhow::Context as AnyhowContext;
use poem_openapi::{param::Query, payload::Html, Object, OpenApi};
use serde::{Deserialize, Serialize};
use std::ops::Sub;

const OPEN_API_HTML: &str = include_str!("../doc/spec.html");

pub struct BasicApi {
    pub context: Arc<Context>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Object)]
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
        duration_secs: Query<Option<u32>>,
    ) -> BasicResult<HealthCheckSuccess> {
        let ledger_info = self.context.get_latest_ledger_info()?;
        if let Some(duration) = duration_secs.0 {
            let timestamp = ledger_info.timestamp();

            let timestamp = Duration::from_micros(timestamp);
            let expectation = SystemTime::now()
                .sub(Duration::from_secs(duration as u64))
                .duration_since(UNIX_EPOCH)
                .context("Failed to determine absolute unix time based on given duration")
                .map_err(BasicError::internal)?;

            if timestamp < expectation {
                return Err(BasicError::internal_str(
                    "The latest ledger info timestamp is less than the expected timestamp",
                ));
            }
        }
        BasicResponse::try_from_rust_value((
            HealthCheckSuccess::new(),
            &ledger_info,
            BasicResponseStatus::Ok,
            &accept_type,
        ))
    }
}
