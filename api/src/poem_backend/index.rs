// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::{AcceptType, ApiTags};
use super::{BasicResponse, BasicResponseStatus, BasicResult};
use crate::context::Context;
use aptos_api_types::IndexResponse;
use poem_openapi::OpenApi;

pub struct IndexApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl IndexApi {
    /// Get ledger info
    ///
    /// Get the latest ledger information, including data such as chain ID,
    /// role type, ledger versions, epoch, etc.
    #[oai(
        path = "/",
        method = "get",
        operation_id = "get_ledger_info",
        tag = "ApiTags::General"
    )]
    async fn get_ledger_info(&self, accept_type: &AcceptType) -> BasicResult<IndexResponse> {
        let ledger_info = self.context.get_latest_ledger_info_poem()?;

        let node_role = self.context.node_role();
        let index_response = IndexResponse::new(ledger_info.clone(), node_role);

        BasicResponse::try_from_rust_value((
            index_response,
            &ledger_info,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }
}
