// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! v2 API router construction.

use super::{
    batch,
    context::V2Context,
    endpoints::{account_transactions, blocks, events, health, modules, resources, transactions, view},
    middleware,
    proxy::{self, V1Proxy},
    websocket,
};
use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;

/// Build the v2 Axum router with all endpoints and middleware.
pub fn build_v2_router(ctx: V2Context) -> Router {
    let content_length_limit = ctx.v2_config.content_length_limit as usize;

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
                .post(transactions::submit_transaction_handler),
        )
        .route(
            "/v2/transactions/:hash",
            get(transactions::get_transaction_handler),
        )
        .route(
            "/v2/transactions/:hash/wait",
            get(transactions::wait_transaction_handler),
        )
        // Account transactions
        .route(
            "/v2/accounts/:address/transactions",
            get(account_transactions::get_account_transactions_handler),
        )
        // Events
        .route(
            "/v2/accounts/:address/events/:creation_number",
            get(events::get_events_handler),
        )
        // View
        .route("/v2/view", post(view::view_handler))
        // Blocks
        .route("/v2/blocks/latest", get(blocks::get_latest_block_handler))
        .route(
            "/v2/blocks/:height",
            get(blocks::get_block_by_height_handler),
        )
        // Batch (JSON-RPC 2.0)
        .route("/v2/batch", post(batch::batch_handler))
        // WebSocket
        .route("/v2/ws", get(websocket::ws_handler))
        // Middleware stack (applied bottom-up: first listed = outermost)
        .layer(axum_middleware::from_fn(middleware::request_id_layer))
        .layer(axum_middleware::from_fn(middleware::logging_layer))
        .layer(middleware::cors_layer())
        .layer(middleware::compression_layer())
        .layer(middleware::size_limit_layer(content_length_limit))
        .with_state(ctx)
}

/// Build a combined router that serves v2 routes and proxies everything
/// else to the internal Poem v1 server. Used for same-port co-hosting.
pub fn build_combined_router(ctx: V2Context, poem_address: SocketAddr) -> Router {
    let v2 = build_v2_router(ctx);
    let v1_proxy = V1Proxy::new(poem_address);

    // v2 routes take priority; anything unmatched falls through to the v1 proxy.
    v2.fallback_service(
        Router::new()
            .fallback(proxy::v1_proxy_fallback)
            .with_state(v1_proxy),
    )
}
