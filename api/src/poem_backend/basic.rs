// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::ApiTags;
use crate::context::Context;
use poem_openapi::{payload::Html, OpenApi};

const OPEN_API_HTML: &str = include_str!("../../doc/spec.html");

pub struct BasicApi {
    pub context: Arc<Context>,
}

// TODO: Consider using swagger UI here instead since it's built in, though
// the UI is much worse. I could look into adding the Elements UI to Poem.

#[OpenApi]
impl BasicApi {
    /// Show OpenAPI explorer
    ///
    /// Provides a UI that you can use to explore the API. You can also retrieve the API directly at `/openapi.yaml` and `/openapi.json`.
    #[oai(
        path = "/spec",
        method = "get",
        operation_id = "openapi",
        tag = "ApiTags::General"
    )]
    async fn openapi(&self) -> Html<String> {
        Html(OPEN_API_HTML.to_string())
    }
}
