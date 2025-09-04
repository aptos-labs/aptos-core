// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Velor Rosetta API
//!
//! [Rosetta API Spec](https://www.rosetta-api.org/docs/Reference.html)

use crate::{
    block::BlockRetriever,
    common::{handle_request, native_coin, usdc_currency, usdc_testnet_currency, with_context},
    error::{ApiError, ApiResult},
    types::Currency,
};
use velor_config::config::ApiConfig;
use velor_logger::debug;
use velor_types::{account_address::AccountAddress, chain_id::ChainId};
use velor_warp_webserver::{logger, Error, WebServer};
use std::{collections::HashSet, convert::Infallible, sync::Arc};
use tokio::task::JoinHandle;
use warp::{
    http::{HeaderValue, Method, StatusCode},
    reply, Filter, Rejection, Reply,
};

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
    rest_client: Option<Arc<velor_rest_client::Client>>,
    /// ChainId of the chain to connect to
    pub chain_id: ChainId,
    /// Block index cache
    pub block_cache: Option<Arc<BlockRetriever>>,
    /// Set of supported currencies
    pub currencies: HashSet<Currency>,
}

impl RosettaContext {
    pub async fn new(
        rest_client: Option<Arc<velor_rest_client::Client>>,
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

    fn rest_client(&self) -> ApiResult<Arc<velor_rest_client::Client>> {
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

/// Creates HTTP server (warp-based) for Rosetta
pub fn bootstrap(
    chain_id: ChainId,
    api_config: ApiConfig,
    rest_client: Option<velor_rest_client::Client>,
    supported_currencies: HashSet<Currency>,
) -> anyhow::Result<tokio::runtime::Runtime> {
    let runtime = velor_runtimes::spawn_named_runtime("rosetta".into(), None);

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
    rest_client: Option<velor_rest_client::Client>,
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
        api.serve(routes(context)).await;
    });
    Ok(handle)
}

/// Collection of all routes for the server
pub fn routes(
    context: RosettaContext,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone {
    account::routes(context.clone())
        .or(block::block_route(context.clone()))
        .or(construction::combine_route(context.clone()))
        .or(construction::derive_route(context.clone()))
        .or(construction::hash_route(context.clone()))
        .or(construction::metadata_route(context.clone()))
        .or(construction::parse_route(context.clone()))
        .or(construction::payloads_route(context.clone()))
        .or(construction::preprocess_route(context.clone()))
        .or(construction::submit_route(context.clone()))
        .or(network::list_route(context.clone()))
        .or(network::options_route(context.clone()))
        .or(network::status_route(context.clone()))
        .or(health_check_route(context))
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_methods(vec![Method::GET, Method::POST])
                .allow_headers(vec![warp::http::header::CONTENT_TYPE]),
        )
        .with(logger())
        .recover(handle_rejection)
}

/// Handle error codes from warp
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    debug!("Failed with: {:?}", err);
    let body = reply::json(&Error::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("unexpected error: {:?}", err),
    ));
    let mut rep = reply::with_status(body, StatusCode::INTERNAL_SERVER_ERROR).into_response();
    rep.headers_mut()
        .insert("access-control-allow-origin", HeaderValue::from_static("*"));
    Ok(rep)
}

/// These parameters are directly passed onto the underlying rest server for a healthcheck
#[derive(serde::Deserialize)]
struct HealthCheckParams {
    pub duration_secs: Option<u64>,
}

/// Default amount of time the fullnode is accepted to be behind (arbitrarily it's 5 minutes)
const HEALTH_CHECK_DEFAULT_SECS: u64 = 300;

pub fn health_check_route(
    server_context: RosettaContext,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("-" / "healthy")
        .and(warp::path::end())
        .and(warp::query().map(move |params: HealthCheckParams| params))
        .and(with_context(server_context))
        .and_then(handle_request(health_check))
}

/// Calls the underlying REST health check
async fn health_check(
    params: HealthCheckParams,
    server_context: RosettaContext,
) -> ApiResult<&'static str> {
    let rest_client = server_context.rest_client()?;
    let duration_secs = params.duration_secs.unwrap_or(HEALTH_CHECK_DEFAULT_SECS);
    rest_client.health_check(duration_secs).await?;

    Ok("velor-node:ok")
}
