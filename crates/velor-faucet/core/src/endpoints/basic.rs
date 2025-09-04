// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::ApiTags;
use crate::funder::{Funder, FunderTrait};
use poem::http::StatusCode;
use poem_openapi::{
    payload::{Html, PlainText},
    OpenApi,
};
use std::sync::Arc;
use tokio::sync::Semaphore;

const OPEN_API_HTML: &str = include_str!("../../../doc/spec.html");

pub struct BasicApi {
    pub concurrent_requests_semaphore: Option<Arc<Semaphore>>,
    pub funder: Arc<Funder>,
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

    /// Check API health
    ///
    /// Basic endpoint that always returns Ok for health.
    #[oai(
        path = "/",
        method = "get",
        operation_id = "root",
        tag = "ApiTags::General"
    )]
    async fn root(&self) -> poem::Result<PlainText<String>> {
        // Confirm that we haven't hit the max concurrent requests.
        if let Some(ref semaphore) = self.concurrent_requests_semaphore {
            if semaphore.available_permits() == 0 {
                return Err(poem::Error::from((
                    StatusCode::SERVICE_UNAVAILABLE,
                    anyhow::anyhow!("Server is overloaded"),
                )));
            }
        }

        // Confirm that the Funder is healthy.
        let funder_health = self.funder.is_healthy().await;
        if !funder_health.can_process_requests {
            return Err(poem::Error::from((
                StatusCode::SERVICE_UNAVAILABLE,
                anyhow::anyhow!(
                    "{}",
                    funder_health
                        .message
                        .unwrap_or_else(|| "Funder is unhealthy".to_string())
                ),
            )));
        }

        Ok(PlainText("tap:ok".to_string()))
    }
}
