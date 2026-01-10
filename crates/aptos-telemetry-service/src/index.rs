// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    auth,
    constants::GCP_CLOUD_TRACE_CONTEXT_HEADER,
    context::Context,
    custom_contract_auth, custom_contract_ingest, custom_event, debug,
    errors::ServiceError,
    log_ingest,
    metrics::SERVICE_ERROR_COUNTS,
    prometheus_push_metrics, remote_config,
    types::response::{ErrorResponse, HealthResponse, IndexResponse},
};
use std::convert::Infallible;
use warp::{
    body::BodyDeserializeError,
    filters::BoxedFilter,
    http::StatusCode,
    reject::{
        InvalidHeader, LengthRequired, MethodNotAllowed, PayloadTooLarge, UnsupportedMediaType,
    },
    reply, Filter, Rejection, Reply,
};

pub fn routes(
    context: Context,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone {
    let v1_api_prefix = warp::path!("api" / "v1" / ..);

    let v1_api = v1_api_prefix.and(
        index(context.clone())
            .or(health())
            .or(auth::check_chain_access(context.clone()))
            .or(auth::auth(context.clone()))
            .or(custom_event::custom_event_ingest(context.clone()))
            .or(prometheus_push_metrics::metrics_ingest(context.clone()))
            .or(log_ingest::log_ingest(context.clone()))
            .or(remote_config::telemetry_log_env(context.clone()))
            // custom contract auth endpoints
            .or(custom_contract_auth::auth_challenge(context.clone()))
            .or(custom_contract_auth::auth(context.clone()))
            .or(custom_contract_ingest::metrics_ingest(context.clone()))
            .or(custom_contract_ingest::log_ingest(context.clone()))
            .or(custom_contract_ingest::custom_event_ingest(context)),
    );

    v1_api
        .recover(handle_rejection)
        .with(warp::trace::trace(|info| {
            let trace_id = info.request_headers()
                .get(GCP_CLOUD_TRACE_CONTEXT_HEADER)
                .and_then(|header_value| header_value.to_str().ok().and_then(|trace_value| trace_value.split_once('/').map(|parts| parts.0)))
                .unwrap_or_default();
            let span = tracing::debug_span!("request", method=%info.method(), path=%info.path(), trace_id=trace_id);
            span
        }))
}

fn index(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path::end()
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_index)
        .boxed()
}

async fn handle_index(context: Context) -> anyhow::Result<impl Reply, Rejection> {
    let resp_payload = IndexResponse {
        public_key: context.noise_config().public_key(),
    };
    Ok(reply::json(&resp_payload))
}

/// Health check endpoint for liveness/readiness probes
/// GET /api/v1/health
fn health() -> BoxedFilter<(impl Reply,)> {
    warp::path!("health")
        .and(warp::get())
        .map(|| {
            reply::json(&HealthResponse {
                status: "ok".to_string(),
            })
        })
        .boxed()
}

pub async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let code;
    let body;

    if let Some(error) = err.find::<ServiceError>() {
        code = error.http_status_code();
        body = reply::json(&ErrorResponse::from(error));

        SERVICE_ERROR_COUNTS
            .with_label_values(&[&format!("{:?}", error.error_code())])
            .inc();
    } else if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        body = reply::json(&ErrorResponse::new(code, "Not Found".to_owned()));
    } else if let Some(cause) = err.find::<BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        body = reply::json(&ErrorResponse::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<InvalidHeader>() {
        code = StatusCode::BAD_REQUEST;
        body = reply::json(&ErrorResponse::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<LengthRequired>() {
        code = StatusCode::LENGTH_REQUIRED;
        body = reply::json(&ErrorResponse::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<PayloadTooLarge>() {
        code = StatusCode::PAYLOAD_TOO_LARGE;
        body = reply::json(&ErrorResponse::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<UnsupportedMediaType>() {
        code = StatusCode::UNSUPPORTED_MEDIA_TYPE;
        body = reply::json(&ErrorResponse::new(code, cause.to_string()));
    } else if let Some(cause) = err.find::<MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        body = reply::json(&ErrorResponse::new(code, cause.to_string()));
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        body = reply::json(&ErrorResponse::new(
            code,
            format!("unexpected error: {:?}", err),
        ));
    }

    debug!("returning an error with status code {}: {:?}", code, err);

    Ok(reply::with_status(body, code).into_response())
}
