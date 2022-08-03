// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::ApiTags;
use crate::context::Context;
use poem_openapi::{payload::Html, OpenApi};

const OPEN_API_HTML: &str = include_str!("../../doc/v1/spec.html");

pub struct BasicApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl BasicApi {
    /// Show OpenAPI explorer
    ///
    /// Provides a UI that you can use to explore the API. You can also retrieve the API directly at `/spec.yaml` and `/spec.json`.
    #[oai(
        path = "/spec",
        method = "get",
        operation_id = "spec",
        tag = "ApiTags::General"
    )]
    async fn spec(&self) -> Html<String> {
        Html(OPEN_API_HTML.to_string())
    }
}
