// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    context::{api_spawn_blocking, Context},
    response::{BasicResponse, BasicResponseStatus, BasicResult},
    ApiTags,
};
use aptos_api_types::{IndexResponse, IndexResponseBcs};
use poem_openapi::OpenApi;
use std::sync::Arc;

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

        api_spawn_blocking(move || match accept_type {
            AcceptType::Json => {
                let index_response = IndexResponse::new(
                    ledger_info.clone(),
                    node_role,
                    Some(aptos_build_info::get_git_hash()),
                );
                BasicResponse::try_from_json((
                    index_response,
                    &ledger_info,
                    BasicResponseStatus::Ok,
                ))
            },
            AcceptType::Bcs => {
                let index_response = IndexResponseBcs::new(ledger_info.clone(), node_role);
                BasicResponse::try_from_bcs((index_response, &ledger_info, BasicResponseStatus::Ok))
            },
        })
        .await
    }
}
