// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    context::Context,
    error_converter_axum,
    middleware_axum,
    routes_axum,
    spec::get_spec,
};
use anyhow::{anyhow, Context as AnyhowContext};
use aptos_config::config::NodeConfig;
use axum::{
    extract::State,
    http::Method,
    middleware,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, runtime::Handle};
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};

pub fn attach_axum_to_runtime(
    runtime_handle: &Handle,
    context: Context,
    config: &NodeConfig,
    random_port: bool,
    port_tx: Option<futures::channel::oneshot::Sender<u16>>,
) -> anyhow::Result<SocketAddr> {
    let context = Arc::new(context);
    let size_limit = context.content_length_limit();

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
            .allow_headers(Any)
            .allow_origin(Any);

        let v1_router = build_v1_router(context.clone(), spec_json, spec_yaml);

        let app = Router::new()
            .route("/", get(routes_axum::root_handler))
            .nest("/v1", v1_router)
            .layer(cors)
            .layer(middleware::from_fn(middleware_axum::logging_middleware))
            .layer(CatchPanicLayer::custom(error_converter_axum::handle_panic));

        if config.api.compression_enabled {
            let app = app.layer(CompressionLayer::new());
            axum::serve(listener, app)
                .await
                .map_err(anyhow::Error::msg)
        } else {
            axum::serve(listener, app)
                .await
                .map_err(anyhow::Error::msg)
        }
    });

    aptos_logger::info!("API server (Axum) is running at {}", actual_address);

    Ok(actual_address)
}

fn build_v1_router(context: Arc<Context>, spec_json: String, spec_yaml: String) -> Router {
    let size_limit = context.content_length_limit();

    Router::new()
        // BasicApi
        .route("/", get(routes_axum::get_ledger_info_handler))
        .route("/spec", get(routes_axum::spec_handler))
        .route("/info", get(routes_axum::info_handler))
        .route("/-/healthy", get(routes_axum::healthy_handler))
        // Spec endpoints
        .route(
            "/spec.json",
            get(routes_axum::spec_json_handler).with_state(spec_json),
        )
        .route(
            "/spec.yaml",
            get(routes_axum::spec_yaml_handler).with_state(spec_yaml),
        )
        // AccountsApi
        .route("/accounts/:address", get(routes_axum::get_account_handler))
        .route(
            "/accounts/:address/resources",
            get(routes_axum::get_account_resources_handler),
        )
        .route(
            "/accounts/:address/balance/:asset_type",
            get(routes_axum::get_account_balance_handler),
        )
        .route(
            "/accounts/:address/modules",
            get(routes_axum::get_account_modules_handler),
        )
        // BlocksApi
        .route(
            "/blocks/by_height/:block_height",
            get(routes_axum::get_block_by_height_handler),
        )
        .route(
            "/blocks/by_version/:version",
            get(routes_axum::get_block_by_version_handler),
        )
        // EventsApi
        .route(
            "/accounts/:address/events/:creation_number",
            get(routes_axum::get_events_by_creation_number_handler),
        )
        .route(
            "/accounts/:address/events/:event_handle/:field_name",
            get(routes_axum::get_events_by_event_handle_handler),
        )
        // StateApi
        .route(
            "/accounts/:address/resource/:resource_type",
            get(routes_axum::get_account_resource_handler),
        )
        .route(
            "/accounts/:address/module/:module_name",
            get(routes_axum::get_account_module_handler),
        )
        .route(
            "/tables/:table_handle/item",
            post(routes_axum::get_table_item_handler),
        )
        .route(
            "/tables/:table_handle/raw_item",
            post(routes_axum::get_raw_table_item_handler),
        )
        .route(
            "/experimental/state_values/raw",
            post(routes_axum::get_raw_state_value_handler),
        )
        // TransactionsApi
        .route(
            "/transactions",
            get(routes_axum::get_transactions_handler)
                .post(routes_axum::submit_transaction_handler),
        )
        .route(
            "/transactions/by_hash/:txn_hash",
            get(routes_axum::get_transaction_by_hash_handler),
        )
        .route(
            "/transactions/wait_by_hash/:txn_hash",
            get(routes_axum::wait_transaction_by_hash_handler),
        )
        .route(
            "/transactions/by_version/:txn_version",
            get(routes_axum::get_transaction_by_version_handler),
        )
        .route(
            "/transactions/auxiliary_info",
            get(routes_axum::get_transactions_auxiliary_info_handler),
        )
        .route(
            "/accounts/:address/transactions",
            get(routes_axum::get_accounts_transactions_handler),
        )
        .route(
            "/accounts/:address/transaction_summaries",
            get(routes_axum::get_accounts_transaction_summaries_handler),
        )
        .route(
            "/transactions/batch",
            post(routes_axum::submit_transactions_batch_handler),
        )
        .route(
            "/transactions/simulate",
            post(routes_axum::simulate_transaction_handler),
        )
        .route(
            "/transactions/encode_submission",
            post(routes_axum::encode_submission_handler),
        )
        .route(
            "/estimate_gas_price",
            get(routes_axum::estimate_gas_price_handler),
        )
        // ViewFunctionApi
        .route("/view", post(routes_axum::view_function_handler))
        // Failpoints
        .route(
            "/set_failpoint",
            get(routes_axum::set_failpoint_handler),
        )
        .with_state(context)
        .layer(middleware::from_fn(move |req, next| {
            middleware_axum::post_size_limit_middleware(size_limit, req, next)
        }))
        .fallback(error_converter_axum::handle_404)
}
