// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{context::Context, error_converter_axum, middleware_axum, routes_axum, spec::get_spec};
use anyhow::{anyhow, Context as AnyhowContext};
use aptos_config::config::NodeConfig;
use axum::{
    http::{Method, StatusCode},
    middleware,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, runtime::Handle};
use tower_http::{catch_panic::CatchPanicLayer, compression::CompressionLayer, cors::CorsLayer};

pub fn attach_axum_to_runtime(
    runtime_handle: &Handle,
    context: Context,
    config: &NodeConfig,
    random_port: bool,
    port_tx: Option<futures::channel::oneshot::Sender<u16>>,
) -> anyhow::Result<SocketAddr> {
    let context = Arc::new(context);

    let api_service = crate::runtime::get_api_service(context.clone());
    let spec_json = get_spec(&api_service, false);
    let spec_yaml = get_spec(&api_service, true);

    let config = config.clone();
    let mut address = config.api.address;

    if random_port {
        address.set_port(0);
    }

    let listener = tokio::task::block_in_place(move || {
        runtime_handle.block_on(async move {
            TcpListener::bind(address)
                .await
                .with_context(|| format!("Failed to bind to address: {}", address))
        })
    })?;

    let actual_address = listener
        .local_addr()
        .context("Failed to get local addr for Axum webserver")?;

    if let Some(port_tx) = port_tx {
        port_tx
            .send(actual_address.port())
            .map_err(|_| anyhow!("Failed to send port"))?;
    }

    runtime_handle.spawn(async move {
        let cors = CorsLayer::new()
            .allow_credentials(true)
            .allow_methods(vec![Method::GET, Method::POST])
            .allow_headers(vec![
                axum::http::header::CONTENT_TYPE,
                axum::http::header::ACCEPT,
                axum::http::header::AUTHORIZATION,
                axum::http::header::ORIGIN,
            ])
            .allow_origin(tower_http::cors::AllowOrigin::mirror_request());

        let app = build_full_router(context.clone(), spec_json, spec_yaml)
            .layer(cors)
            .layer(middleware::from_fn(middleware_axum::logging_middleware))
            .layer(CatchPanicLayer::custom(error_converter_axum::handle_panic));

        // Apply compression if enabled
        let app = if config.api.compression_enabled {
            app.layer(CompressionLayer::new())
        } else {
            app
        };

        axum::serve(listener, app).await.map_err(anyhow::Error::msg)
    });

    aptos_logger::info!("API server (Axum) is running at {}", actual_address);

    Ok(actual_address)
}

fn build_full_router(context: Arc<Context>, spec_json: String, spec_yaml: String) -> Router {
    let size_limit = context.content_length_limit();

    let spec_json_clone = spec_json.clone();
    let spec_yaml_clone = spec_yaml.clone();

    Router::new()
        .route("/", get(routes_axum::root_handler))
        // Both /v1 and /v1/ should return the ledger info
        .route("/v1", get(routes_axum::get_ledger_info_handler))
        .route("/v1/", get(routes_axum::get_ledger_info_handler))
        .route("/v1/spec", get(routes_axum::spec_handler))
        .route("/v1/info", get(routes_axum::info_handler))
        .route("/v1/-/healthy", get(routes_axum::healthy_handler))
        .route(
            "/v1/spec.json",
            get(move || async move {
                (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    spec_json_clone,
                )
            }),
        )
        .route(
            "/v1/spec.yaml",
            get(move || async move {
                (
                    StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/x-yaml")],
                    spec_yaml_clone,
                )
            }),
        )
        // AccountsApi
        .route(
            "/v1/accounts/:address",
            get(routes_axum::get_account_handler),
        )
        .route(
            "/v1/accounts/:address/resources",
            get(routes_axum::get_account_resources_handler),
        )
        .route(
            "/v1/accounts/:address/balance/:asset_type",
            get(routes_axum::get_account_balance_handler),
        )
        .route(
            "/v1/accounts/:address/modules",
            get(routes_axum::get_account_modules_handler),
        )
        // BlocksApi
        .route(
            "/v1/blocks/by_height/:block_height",
            get(routes_axum::get_block_by_height_handler),
        )
        .route(
            "/v1/blocks/by_version/:version",
            get(routes_axum::get_block_by_version_handler),
        )
        // EventsApi
        .route(
            "/v1/accounts/:address/events/:creation_number",
            get(routes_axum::get_events_by_creation_number_handler),
        )
        .route(
            "/v1/accounts/:address/events/:event_handle/:field_name",
            get(routes_axum::get_events_by_event_handle_handler),
        )
        // StateApi
        .route(
            "/v1/accounts/:address/resource/:resource_type",
            get(routes_axum::get_account_resource_handler),
        )
        .route(
            "/v1/accounts/:address/module/:module_name",
            get(routes_axum::get_account_module_handler),
        )
        .route(
            "/v1/tables/:table_handle/item",
            post(routes_axum::get_table_item_handler),
        )
        .route(
            "/v1/tables/:table_handle/raw_item",
            post(routes_axum::get_raw_table_item_handler),
        )
        .route(
            "/v1/experimental/state_values/raw",
            post(routes_axum::get_raw_state_value_handler),
        )
        // TransactionsApi
        .route(
            "/v1/transactions",
            get(routes_axum::get_transactions_handler)
                .post(routes_axum::submit_transaction_handler),
        )
        .route(
            "/v1/transactions/by_hash/:txn_hash",
            get(routes_axum::get_transaction_by_hash_handler),
        )
        .route(
            "/v1/transactions/wait_by_hash/:txn_hash",
            get(routes_axum::wait_transaction_by_hash_handler),
        )
        .route(
            "/v1/transactions/by_version/:txn_version",
            get(routes_axum::get_transaction_by_version_handler),
        )
        .route(
            "/v1/transactions/auxiliary_info",
            get(routes_axum::get_transactions_auxiliary_info_handler),
        )
        .route(
            "/v1/accounts/:address/transactions",
            get(routes_axum::get_accounts_transactions_handler),
        )
        .route(
            "/v1/accounts/:address/transaction_summaries",
            get(routes_axum::get_accounts_transaction_summaries_handler),
        )
        .route(
            "/v1/transactions/batch",
            post(routes_axum::submit_transactions_batch_handler),
        )
        .route(
            "/v1/transactions/simulate",
            post(routes_axum::simulate_transaction_handler),
        )
        .route(
            "/v1/transactions/encode_submission",
            post(routes_axum::encode_submission_handler),
        )
        .route(
            "/v1/estimate_gas_price",
            get(routes_axum::estimate_gas_price_handler),
        )
        // ViewFunctionApi
        .route("/v1/view", post(routes_axum::view_function_handler))
        // Failpoints
        .route("/v1/set_failpoint", get(routes_axum::set_failpoint_handler))
        .with_state(context)
        .layer(middleware::from_fn(move |req, next| {
            middleware_axum::post_size_limit_middleware(size_limit, req, next)
        }))
        .fallback(error_converter_axum::handle_404)
}
