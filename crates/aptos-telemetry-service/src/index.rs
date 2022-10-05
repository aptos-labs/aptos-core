// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    auth,
    constants::{GCP_CLOUD_TRACE_CONTEXT_HEADER, LOG_TRACE_FIELD},
    context::Context,
    custom_event,
    error::ServiceError,
    log_ingest, prometheus_push_metrics, remote_config,
    types::index::IndexResponse,
};
use std::convert::Infallible;
use tracing::debug;
use warp::{
    body::BodyDeserializeError,
    filters::BoxedFilter,
    http::StatusCode,
    reject::{
        InvalidHeader, LengthRequired, MethodNotAllowed, PayloadTooLarge, UnsupportedMediaType,
    },
    reply, Filter, Rejection, Reply,
};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Infallible> + Clone {
    let v1_api_prefix = warp::path!("api" / "v1" / ..);

    let v1_api = v1_api_prefix.and(
        index(context.clone())
            .or(auth::check_chain_access(context.clone()))
            .or(auth::auth(context.clone()))
            .or(custom_event::custom_event_ingest(context.clone()))
            .or(prometheus_push_metrics::metrics_ingest(context.clone()))
            .or(log_ingest::log_ingest(context.clone()))
            .or(remote_config::telemetry_log_env(context.clone())),
    );

    let legacy_api = index_legacy(context.clone())
        .or(auth::check_chain_access(context.clone()))
        .or(auth::auth(context.clone()))
        .or(custom_event::custom_event_legacy(context.clone()))
        .or(prometheus_push_metrics::metrics_ingest_legacy(
            context.clone(),
        ))
        .or(log_ingest::log_ingest_legacy(context));

    legacy_api
        .or(v1_api)
        .recover(handle_rejection)
        .with(warp::trace::trace(|info| {
            let span = tracing::debug_span!("request", method=%info.method(), path=%info.path());
            if let Some(header_value) = info.request_headers().get(GCP_CLOUD_TRACE_CONTEXT_HEADER) {
                span.record(LOG_TRACE_FIELD, header_value.to_str().unwrap_or_default());
            }
            span
        }))
}

/// TODO: Cleanup after v1 API is ramped up
fn index_legacy(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path::end()
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_index_legacy)
        .boxed()
}

fn index(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path::end()
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_index)
        .boxed()
}

async fn handle_index_legacy(context: Context) -> anyhow::Result<impl Reply, Rejection> {
    let resp = reply::json(&context.noise_config().public_key());
    Ok(resp)
}

async fn handle_index(context: Context) -> anyhow::Result<impl Reply, Rejection> {
    let resp_payload = IndexResponse {
        public_key: context.noise_config().public_key(),
    };
    Ok(reply::json(&resp_payload))
}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let code;
    let body;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        body = reply::json(&ServiceError::new(code, "Not Found".to_owned()));
    } else if let Some(error) = err.find::<ServiceError>() {
        code = error.status_code();
        body = reply::json(error);
    } else if let Some(cause) = err.find::<BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<InvalidHeader>() {
        code = StatusCode::BAD_REQUEST;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<LengthRequired>() {
        code = StatusCode::LENGTH_REQUIRED;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<PayloadTooLarge>() {
        code = StatusCode::PAYLOAD_TOO_LARGE;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<UnsupportedMediaType>() {
        code = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        body = reply::json(&ServiceError::new(code, cause.to_string()));
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        body = reply::json(&ServiceError::new(
            code,
            format!("unexpected error: {:?}", err),
        ));
    }

    debug!("returning an error with status code {}: {:?}", code, err);

    Ok(reply::with_status(body, code).into_response())
}
