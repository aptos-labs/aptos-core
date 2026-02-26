// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    context::Context,
    response_axum::{AptosErrorResponse, AptosResponse},
};
use aptos_api_types::{IndexResponse, IndexResponseBcs};
use std::sync::Arc;

/// Framework-agnostic business logic for the ledger info endpoint.
/// Called by the Axum handler directly, bypassing the Poem bridge.
pub fn get_ledger_info_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
) -> Result<AptosResponse<IndexResponse>, AptosErrorResponse> {
    context.check_api_output_enabled::<AptosErrorResponse>("Get ledger info", accept_type)?;
    let ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;
    let node_role = context.node_role();
    let encryption_key_hex = context
        .get_encryption_key(ledger_info.version())
        .unwrap_or(None)
        .map(hex::encode);

    match accept_type {
        AcceptType::Json => {
            let index_response = IndexResponse::new(
                ledger_info.clone(),
                node_role,
                Some(aptos_build_info::get_git_hash()),
                encryption_key_hex,
            );
            AptosResponse::try_from_json(index_response, &ledger_info)
        },
        AcceptType::Bcs => {
            let index_response = IndexResponseBcs::new(ledger_info.clone(), node_role);
            AptosResponse::try_from_bcs(index_response, &ledger_info)
        },
    }
}
