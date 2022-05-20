// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_api::runtime::WebServer;
use aptos_config::config::ApiConfig;
use aptos_rest_client::aptos_api_types::Error;
use std::convert::Infallible;
use warp::{
    filters::BoxedFilter,
    http::{HeaderValue, Method, StatusCode},
    reject::{MethodNotAllowed, PayloadTooLarge, UnsupportedMediaType},
    reply, Filter, Rejection, Reply,
};

/// Rosetta API context for use on all APIs
#[derive(Clone, Debug)]
pub struct RosettaContext {
    pub rest_client: aptos_rest_client::Client,
}

impl RosettaContext {
    pub fn filter(self) -> impl Filter<Extract = (Self,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }
}

/// Creates HTTP server (warp-based) serves for Rosetta
pub fn bootstrap(
    api_config: ApiConfig,
    rest_client: aptos_rest_client::Client,
) -> anyhow::Result<tokio::runtime::Runtime> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("rosetta")
        .enable_all()
        .build()
        .expect("[rosetta] failed to create runtime");

    let api = WebServer::from(api_config.clone());

    runtime.spawn(async move {
        let context = RosettaContext { rest_client };
        let routes = routes(context);
        api.serve(routes).await;
    });
    Ok(runtime)
}

pub fn routes(
    context: RosettaContext,
) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    index(context)
        // TODO: Add open api spec
        // TODO: Add health check
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_methods(vec![Method::GET, Method::POST])
                .allow_headers(vec![warp::http::header::CONTENT_TYPE]),
        )
        .recover(handle_rejection)
    // TODO Logger
    // TODO metrics
}

// GET /
pub fn index(context: RosettaContext) -> BoxedFilter<(impl Reply,)> {
    warp::path::end()
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_index)
        .boxed()
}

pub async fn handle_index(context: RosettaContext) -> Result<impl Reply, Rejection> {
    Ok("Hello")
}

const OPEN_API_HTML: &str = include_str!("../../../api/doc/spec.html");
const OPEN_API_SPEC: &str = include_str!("../../../api/doc/openapi.yaml");

fn open_api_html() -> String {
    OPEN_API_HTML.replace("hideTryIt=\"true\"", "")
}

// GET /openapi.yaml
// GET /spec.html
pub fn openapi_spec() -> BoxedFilter<(impl Reply,)> {
    let spec = warp::path!("openapi.yaml")
        .and(warp::get())
        .map(|| OPEN_API_SPEC)
        .boxed();
    let html = warp::path!("spec.html")
        .and(warp::get())
        .map(|| reply::html(open_api_html()))
        .boxed();
    spec.or(html).boxed()
}

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
