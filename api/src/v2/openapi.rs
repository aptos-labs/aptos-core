// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! OpenAPI spec generation for the v2 API.
//!
//! Uses `utoipa` derive macros on handlers and types, then serves
//! the spec as JSON and YAML at `/v2/spec.json` and `/v2/spec.yaml`.

use crate::v2::{
    endpoints::{
        account_transactions, accounts, balance, blocks, events, gas_estimation, health, modules,
        resources, simulate, tables, transactions, view,
    },
    error::{ErrorCode, V2Error},
    types::{HealthResponse, LedgerMetadata, NodeInfo},
};
#[cfg(feature = "api-v2-sse")]
use crate::v2::endpoints::sse;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use utoipa::OpenApi;

/// The core OpenAPI spec for the Aptos v2 REST API (always available).
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Aptos Node API v2",
        version = "2.0.0",
        description = "REST API v2 for the Aptos blockchain. Served at /v2.",
        license(name = "Apache-2.0"),
    ),
    paths(
        // Health
        health::health_handler,
        health::info_handler,
        // Accounts
        accounts::get_account_handler,
        // Balance
        balance::get_balance_handler,
        // Resources
        resources::get_resources_handler,
        resources::get_resource_handler,
        // Modules
        modules::get_modules_handler,
        modules::get_module_handler,
        // Transactions
        transactions::list_transactions_handler,
        transactions::get_transaction_handler,
        transactions::get_transaction_by_version_handler,
        transactions::submit_transaction_handler,
        transactions::wait_transaction_handler,
        // Simulate
        simulate::simulate_transaction_handler,
        // Gas estimation
        gas_estimation::estimate_gas_price_handler,
        // Account transactions
        account_transactions::get_account_transactions_handler,
        // Events
        events::get_events_handler,
        // View
        view::view_handler,
        // Tables
        tables::get_table_item_handler,
        // Blocks
        blocks::get_block_by_height_handler,
        blocks::get_block_by_version_handler,
        blocks::get_latest_block_handler,
    ),
    components(schemas(
        // Response envelope & metadata
        LedgerMetadata,
        HealthResponse,
        NodeInfo,
        // Error types
        V2Error,
        ErrorCode,
        // Transaction types
        transactions::SubmitResult,
        account_transactions::TransactionSummary,
        // Balance
        balance::BalanceResponse,
    )),
    tags(
        (name = "Health", description = "Node health and info"),
        (name = "Accounts", description = "Account info, resources, modules, and transactions"),
        (name = "Transactions", description = "Transaction listing, lookup, submission, simulation, and waiting"),
        (name = "Events", description = "On-chain event queries"),
        (name = "View", description = "Execute view functions"),
        (name = "Tables", description = "Table item queries"),
        (name = "Blocks", description = "Block queries"),
    )
)]
struct V2ApiDocCore;

/// SSE endpoints for the OpenAPI spec (only available with `api-v2-sse`).
#[cfg(feature = "api-v2-sse")]
#[derive(OpenApi)]
#[openapi(
    paths(
        sse::sse_blocks_handler,
        sse::sse_events_handler,
    ),
    tags(
        (name = "SSE", description = "Server-Sent Events streaming endpoints"),
    )
)]
struct V2ApiDocSse;

/// Combined v2 OpenAPI spec. Merges the core spec with optional SSE/WebSocket
/// docs based on which features are compiled in.
pub struct V2ApiDoc;

impl V2ApiDoc {
    pub fn openapi() -> utoipa::openapi::OpenApi {
        let mut spec = V2ApiDocCore::openapi();

        #[cfg(feature = "api-v2-sse")]
        {
            let sse_spec = V2ApiDocSse::openapi();
            spec.merge(sse_spec);
        }

        spec
    }
}

/// GET /v2/spec.json -- serve the OpenAPI spec as JSON.
pub async fn spec_json_handler() -> impl IntoResponse {
    Json(V2ApiDoc::openapi())
}

/// GET /v2/spec.yaml -- serve the OpenAPI spec as YAML.
pub async fn spec_yaml_handler() -> Response {
    match V2ApiDoc::openapi().to_yaml() {
        Ok(yaml) => (
            StatusCode::OK,
            [("content-type", "text/yaml; charset=utf-8")],
            yaml,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to generate YAML spec: {}", e),
        )
            .into_response(),
    }
}
