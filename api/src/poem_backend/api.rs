// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;
use aptos_api_types::IndexResponse;
use poem::{http::StatusCode, Error as PoemError, Result as PoemResult};
use poem_openapi::{
    payload::{Html, Json},
    OpenApi, Tags,
};

const OPEN_API_HTML: &str = include_str!("../../doc/spec.html");

pub struct Api {
    context: Context,
}

impl Api {
    pub fn new(context: Context) -> Self {
        Self { context }
    }
}

// TODO: Move these impls throughout each of the files in the parent directory.
// The only reason I do it here right now is the existing handler functions return
// opaque reply objects and therefore I can't re-use them, so I'd have to pollute
// those files with these impls below.

// TODO: Consider using swagger UI here instead since it's built in, though
// the UI is much worse. I could look into adding the Elements UI to Poem.

#[derive(Tags)]
enum ApiTags {
    /// General information.
    General,
}

#[OpenApi]
impl Api {
    /// get_ledger_info
    ///
    /// Get the latest ledger information, including data such as chain ID, role type, ledger versions, epoch, etc.
    #[oai(
        path = "/",
        method = "get",
        operation_id = "get_ledger_info",
        tag = "ApiTags::General"
    )]
    async fn get_ledger_info(&self) -> PoemResult<Json<IndexResponse>> {
        let ledger_info = self.context.get_latest_ledger_info().map_err(|e| {
            PoemError::from((StatusCode::INTERNAL_SERVER_ERROR, anyhow::anyhow!(e)))
        })?;
        let node_role = self.context.node_role();
        let index_response = IndexResponse::new(ledger_info, node_role);
        Ok(Json(index_response))
    }

    /// openapi
    ///
    /// Provides a UI that you can use to explore the API. You can also retrieve the API directly at `/openapi.yaml` and `/openapi.json`.
    #[oai(
        path = "/openapi",
        method = "get",
        operation_id = "openapi",
        tag = "ApiTags::General"
    )]
    async fn openapi(&self) -> Html<String> {
        Html(OPEN_API_HTML.to_string())
    }
}
