// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Aptos Rosetta API
//!
//! [Rosetta API Spec](https://www.rosetta-api.org/docs/Reference.html)

use crate::{
    block::BlockRetriever,
    common::{into_rosetta_response, native_coin, usdc_currency, usdc_testnet_currency},
    error::{ApiError, ApiResult},
    types::Currency,
};
use axum::{
    extract::Query,
    http::{header::CONTENT_TYPE, Method},
    response::IntoResponse,
    response::Response,
    routing::{get, post},
    Json,
    Router,
};
use aptos_config::config::ApiConfig;
use aptos_logger::debug;
use aptos_types::chain_id::ChainId;
use aptos_warp_webserver::{logger, WebServer};
use std::{collections::HashSet, sync::Arc};
use tokio::task::JoinHandle;
use tower_http::cors::{Any, CorsLayer};

pub use aptos_types::account_address::AccountAddress;

mod account;
mod block;
mod construction;
mod network;

pub mod client;
pub mod common;
pub mod error;
pub mod types;

#[cfg(test)]
mod test;

pub const NODE_VERSION: &str = "0.1";
pub const ROSETTA_VERSION: &str = "1.4.12";

/// Rosetta API context for use on all APIs
#[derive(Clone, Debug)]
pub struct RosettaContext {
    /// A rest client to connect to a fullnode
    rest_client: Option<Arc<aptos_rest_client::Client>>,
    /// ChainId of the chain to connect to
    pub chain_id: ChainId,
    /// Block index cache
    pub block_cache: Option<Arc<BlockRetriever>>,
    /// Set of supported currencies
    pub currencies: HashSet<Currency>,
}

impl RosettaContext {
    pub async fn new(
        rest_client: Option<Arc<aptos_rest_client::Client>>,
        chain_id: ChainId,
        block_cache: Option<Arc<BlockRetriever>>,
        mut currencies: HashSet<Currency>,
    ) -> Self {
        // Always add APT
        currencies.insert(native_coin());

        // Depending on the chain add appropriate USDC
        if chain_id.is_mainnet() {
            currencies.insert(usdc_currency());
        } else if chain_id.is_testnet() {
            currencies.insert(usdc_testnet_currency());
        }

        RosettaContext {
            rest_client,
            chain_id,
            block_cache,
            currencies,
        }
    }

    fn rest_client(&self) -> ApiResult<Arc<aptos_rest_client::Client>> {
        if let Some(ref client) = self.rest_client {
            Ok(client.clone())
        } else {
            Err(ApiError::NodeIsOffline)
        }
    }

    fn block_cache(&self) -> ApiResult<Arc<BlockRetriever>> {
        if let Some(ref block_cache) = self.block_cache {
            Ok(block_cache.clone())
        } else {
            Err(ApiError::NodeIsOffline)
        }
    }
}

/// Creates HTTP server (axum-based) for Rosetta
pub fn bootstrap(
    chain_id: ChainId,
    api_config: ApiConfig,
    rest_client: Option<aptos_rest_client::Client>,
    supported_currencies: HashSet<Currency>,
) -> anyhow::Result<tokio::runtime::Runtime> {
    let runtime = aptos_runtimes::spawn_named_runtime("rosetta".into(), None);

    debug!("Starting up Rosetta server with {:?}", api_config);

    runtime.spawn(bootstrap_async(
        chain_id,
        api_config,
        rest_client,
        supported_currencies,
    ));
    Ok(runtime)
}

/// Creates HTTP server for Rosetta in an async context
pub async fn bootstrap_async(
    chain_id: ChainId,
    api_config: ApiConfig,
    rest_client: Option<aptos_rest_client::Client>,
    supported_currencies: HashSet<Currency>,
) -> anyhow::Result<JoinHandle<()>> {
    debug!("Starting up Rosetta server with {:?}", api_config);

    if let Some(ref client) = rest_client {
        assert_eq!(
            chain_id.id(),
            client
                .get_ledger_information()
                .await
                .expect("Should successfully get ledger information from Rest API on bootstap")
                .into_inner()
                .chain_id,
            "Failed to match Rosetta chain Id to upstream server"
        );
    }

    let api = WebServer::from(api_config.clone());
    let handle = tokio::spawn(async move {
        // If it's Online mode, add the block cache
        let rest_client = rest_client.map(Arc::new);

        // TODO: The BlockRetriever has no cache, and should probably be renamed from block_cache
        let block_cache = rest_client.as_ref().map(|rest_client| {
            Arc::new(BlockRetriever::new(
                api_config.max_transactions_page_size,
                rest_client.clone(),
            ))
        });

        let context = RosettaContext::new(
            rest_client.clone(),
            chain_id,
            block_cache,
            supported_currencies,
        )
        .await;
        if let Err(err) = api.serve(routes(context)).await {
            panic!("Failed to start rosetta service: {}", err);
        }
    });
    Ok(handle)
}

/// Collection of all routes for the server
pub fn routes(context: RosettaContext) -> Router {
    logger(
        Router::new()
        .route("/account/balance", post(account::account_balance_route))
        .route("/block", post(block::block_route))
        .route("/construction/combine", post(construction::combine_route))
        .route("/construction/derive", post(construction::derive_route))
        .route("/construction/hash", post(construction::hash_route))
        .route(
            "/construction/metadata",
            post(construction::metadata_route),
        )
        .route("/construction/parse", post(construction::parse_route))
        .route(
            "/construction/payloads",
            post(construction::payloads_route),
        )
        .route(
            "/construction/preprocess",
            post(construction::preprocess_route),
        )
        .route("/construction/submit", post(construction::submit_route))
        .route("/network/list", post(network::network_list_route))
        .route("/network/options", post(network::network_options_route))
        .route("/network/status", post(network::network_status_route))
        .route("/-/healthy", get(health_check_route))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([CONTENT_TYPE]),
        )
        .with_state(context)
    )
}

/// These parameters are directly passed onto the underlying rest server for a healthcheck
#[derive(serde::Deserialize)]
struct HealthCheckParams {
    pub duration_secs: Option<u64>,
}

/// Default amount of time the fullnode is accepted to be behind (arbitrarily it's 5 minutes)
const HEALTH_CHECK_DEFAULT_SECS: u64 = 300;

async fn health_check_route(
    Query(params): Query<HealthCheckParams>,
    axum::extract::State(context): axum::extract::State<RosettaContext>,
) -> Response {
    match health_check(params, context).await {
        Ok(body) => Json(body).into_response(),
        Err(err) => into_rosetta_response::<&'static str>(Err(err)),
    }
}

/// Calls the underlying REST health check
async fn health_check(
    params: HealthCheckParams,
    server_context: RosettaContext,
) -> ApiResult<&'static str> {
    let rest_client = server_context.rest_client()?;
    let duration_secs = params.duration_secs.unwrap_or(HEALTH_CHECK_DEFAULT_SECS);
    rest_client.health_check(duration_secs).await?;

    Ok("aptos-node:ok")
}
