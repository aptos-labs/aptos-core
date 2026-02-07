// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! v2 API router construction.

use super::{
    context::V2Context,
    endpoints::{blocks, events, health, modules, resources, transactions, view},
};
use axum::{
    routing::{get, post},
    Router,
};

/// Build the v2 Axum router with all endpoints.
pub fn build_v2_router(ctx: V2Context) -> Router {
    Router::new()
        // Health & info
        .route("/v2/health", get(health::health_handler))
        .route("/v2/info", get(health::info_handler))
        // Resources
        .route(
            "/v2/accounts/:address/resources",
            get(resources::get_resources_handler),
        )
        .route(
            "/v2/accounts/:address/resource/*resource_type",
            get(resources::get_resource_handler),
        )
        // Modules
        .route(
            "/v2/accounts/:address/modules",
            get(modules::get_modules_handler),
        )
        .route(
            "/v2/accounts/:address/module/:module_name",
            get(modules::get_module_handler),
        )
        // Transactions
        .route(
            "/v2/transactions",
            get(transactions::list_transactions_handler)
                .post(|| async { "TODO: submit transaction" }),
        )
        .route(
            "/v2/transactions/:hash",
            get(transactions::get_transaction_handler),
        )
        // Events
        .route(
            "/v2/accounts/:address/events/:creation_number",
            get(events::get_events_handler),
        )
        // View
        .route("/v2/view", post(view::view_handler))
        // Blocks
        .route(
            "/v2/blocks/latest",
            get(blocks::get_latest_block_handler),
        )
        .route(
            "/v2/blocks/:height",
            get(blocks::get_block_by_height_handler),
        )
        .with_state(ctx)
}
