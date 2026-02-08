// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! OpenAPI spec generation for the v2 API.
//!
//! Uses `utoipa` derive macros on handlers and types, then serves
//! the spec as JSON and YAML at `/v2/spec.json` and `/v2/spec.yaml`.

use crate::v2::{
    endpoints::{
        account_transactions, blocks, events, health, modules, resources, transactions, view,
    },
    error::{ErrorCode, V2Error},
    types::{HealthResponse, LedgerMetadata, NodeInfo},
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use utoipa::OpenApi;

/// The OpenAPI spec for the Aptos v2 REST API.
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
        // Resources
        resources::get_resources_handler,
        resources::get_resource_handler,
        // Modules
        modules::get_modules_handler,
        modules::get_module_handler,
        // Transactions
        transactions::list_transactions_handler,
        transactions::get_transaction_handler,
        transactions::submit_transaction_handler,
        transactions::wait_transaction_handler,
        // Account transactions
        account_transactions::get_account_transactions_handler,
        // Events
        events::get_events_handler,
        // View
        view::view_handler,
        // Blocks
        blocks::get_block_by_height_handler,
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
    )),
    tags(
        (name = "Health", description = "Node health and info"),
        (name = "Accounts", description = "Account resources, modules, and transactions"),
        (name = "Transactions", description = "Transaction listing, lookup, submission, and waiting"),
        (name = "Events", description = "On-chain event queries"),
        (name = "View", description = "Execute view functions"),
        (name = "Blocks", description = "Block queries"),
    )
)]
pub struct V2ApiDoc;

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
