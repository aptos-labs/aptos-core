// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::ApiTags;
use crate::context::Context;
use aptos_api_types::IndexResponse;
use poem::{http::StatusCode, Error as PoemError, Result as PoemResult};
use poem_openapi::{payload::Json, OpenApi};

pub struct IndexApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl IndexApi {
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
}
