// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::accept_type::AcceptType;
use crate::context::Context;
use crate::response::{BasicResponse, BasicResponseStatus, BasicResult};
use crate::ApiTags;
use aptos_api_types::IndexResponse;
use poem_openapi::OpenApi;

/// API for the index, to retrieve the ledger information
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
    async fn get_ledger_info(&self, accept_type: AcceptType) -> BasicResult<IndexResponse> {
        self.context
            .check_api_output_enabled("Get ledger info", &accept_type)?;
        let ledger_info = self.context.get_latest_ledger_info()?;

        let node_role = self.context.node_role();
        let index_response = IndexResponse::new(ledger_info.clone(), node_role);

        BasicResponse::try_from_rust_value((
            index_response,
            &ledger_info,
            BasicResponseStatus::Ok,
            &accept_type,
        ))
    }
}
