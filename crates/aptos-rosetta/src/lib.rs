// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Aptos Rosetta API
//!
//! [Rosetta API Spec](https://www.rosetta-api.org/docs/Reference.html)

use aptos_api::runtime::WebServer;
use aptos_config::config::ApiConfig;
use aptos_logger::debug;
use aptos_rest_client::aptos_api_types::Error;
use aptos_types::chain_id::ChainId;
use std::convert::Infallible;
use tokio::task::JoinHandle;
use warp::{
    http::{HeaderValue, Method, StatusCode},
    reject::{MethodNotAllowed, PayloadTooLarge, UnsupportedMediaType},
    reply, Filter, Rejection, Reply,
};

mod account;
mod network;

pub mod client;
pub mod common;
pub mod error;
pub mod types;

pub const CURRENCY: &str = "APTOS";
pub const NUM_DECIMALS: u64 = 6;
pub const MIDDLEWARE_VERSION: &str = "1.0.0";
pub const NODE_VERSION: &str = "0.1";
pub const ROSETTA_VERSION: &str = "1.4.12";

/// Rosetta API context for use on all APIs
#[derive(Clone, Debug)]
pub struct RosettaContext {
    pub rest_client: aptos_rest_client::Client,
    pub chain_id: ChainId,
}

/// Creates HTTP server (warp-based) for Rosetta
pub fn bootstrap(
    chain_id: ChainId,
    api_config: ApiConfig,
    rest_client: aptos_rest_client::Client,
) -> anyhow::Result<tokio::runtime::Runtime> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("rosetta")
        .enable_all()
        .build()
        .expect("[rosetta] failed to create runtime");

    debug!("Starting up Rosetta server with {:?}", api_config);
    let api = WebServer::from(api_config);

    runtime.spawn(async move {
        let context = RosettaContext {
            rest_client,
            chain_id,
        };
        api.serve(routes(context)).await;
    });
    Ok(runtime)
}

pub async fn bootstrap_async(
    chain_id: ChainId,
    api_config: ApiConfig,
    rest_client: aptos_rest_client::Client,
) -> anyhow::Result<JoinHandle<()>> {
    debug!("Starting up Rosetta server with {:?}", api_config);
    let api = WebServer::from(api_config);
    let handle = tokio::spawn(async move {
        let context = RosettaContext {
            rest_client,
            chain_id,
        };
        api.serve(routes(context)).await;
    });
    Ok(handle)
}

/// Collection of all routes for the server
pub fn routes(
    context: RosettaContext,
) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    account::routes(context.clone())
        .or(network::routes(context))
        // TODO: Add health check?
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_methods(vec![Method::GET, Method::POST])
                .allow_headers(vec![warp::http::header::CONTENT_TYPE]),
        )
        .recover(handle_rejection)
    // TODO Logger?
    // TODO metrics?
}

/// Handle error codes from warp
async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let body;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        body = reply::json(&Error::new(code, "Not Found".to_owned()));
    } else if let Some(cause) = err.find::<warp::cors::CorsForbidden>() {
        code = StatusCode::FORBIDDEN;
        body = reply::json(&Error::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<warp::body::BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        body = reply::json(&Error::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<warp::reject::LengthRequired>() {
        code = StatusCode::LENGTH_REQUIRED;
        body = reply::json(&Error::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<PayloadTooLarge>() {
        code = StatusCode::PAYLOAD_TOO_LARGE;
        body = reply::json(&Error::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<UnsupportedMediaType>() {
        code = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        body = reply::json(&Error::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        body = reply::json(&Error::new(code, cause.to_string()));
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        body = reply::json(&Error::new(code, format!("unexpected error: {:?}", err)));
    }
    let mut rep = reply::with_status(body, code).into_response();
    rep.headers_mut()
        .insert("access-control-allow-origin", HeaderValue::from_static("*"));
    Ok(rep)
}
