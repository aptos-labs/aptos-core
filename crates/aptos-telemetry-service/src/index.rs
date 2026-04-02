// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    auth,
    constants::GCP_CLOUD_TRACE_CONTEXT_HEADER,
    context::Context,
    custom_contract_auth, custom_contract_ingest, custom_event,
    errors::ServiceError,
    log_ingest,
    metrics::SERVICE_ERROR_COUNTS,
    prometheus_push_metrics, remote_config,
    types::response::{ErrorResponse, HealthResponse, IndexResponse},
};
use axum::{
    extract::{DefaultBodyLimit, Extension, Request},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json as AxumJson, Router,
};

pub fn routes(context: Context) -> Router {
    Router::new()
        .route("/api/v1/", get(get_index))
        .route("/api/v1/health", get(get_health))
        .route(
            "/api/v1/chain-access/:chain_id",
            get(auth::get_chain_access),
        )
        .route("/api/v1/auth", post(auth::post_auth))
        .route(
            "/api/v1/ingest/custom-event",
            post(custom_event::post_custom_event),
        )
        .route(
            "/api/v1/ingest/metrics",
            post(prometheus_push_metrics::post_metrics_ingest),
        )
        .route("/api/v1/ingest/logs", post(log_ingest::post_log_ingest))
        .route(
            "/api/v1/config/env/telemetry-log",
            get(remote_config::get_telemetry_log_env),
        )
        .route(
            "/api/v1/custom-contract/:contract_name/auth-challenge",
            post(custom_contract_auth::post_auth_challenge),
        )
        .route(
            "/api/v1/custom-contract/:contract_name/auth",
            post(custom_contract_auth::post_custom_auth),
        )
        .route(
            "/api/v1/custom-contract/:contract_name/ingest/metrics",
            post(custom_contract_ingest::post_custom_contract_metrics),
        )
        .route(
            "/api/v1/custom-contract/:contract_name/ingest/logs",
            post(custom_contract_ingest::post_custom_contract_logs),
        )
        .route(
            "/api/v1/custom-contract/:contract_name/ingest/custom-event",
            post(custom_contract_ingest::post_custom_contract_custom_event),
        )
        .fallback(fallback_not_found)
        .layer(middleware::from_fn(normalize_axum_error_responses))
        .layer(DefaultBodyLimit::max(
            crate::constants::MAX_CONTENT_LENGTH as usize,
        ))
        .layer(middleware::from_fn(trace_middleware))
        .layer(Extension(context))
}

async fn trace_middleware(request: Request, next: Next) -> Response {
    let trace_id = request
        .headers()
        .get(GCP_CLOUD_TRACE_CONTEXT_HEADER)
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|trace_value| trace_value.split_once('/').map(|parts| parts.0))
        .unwrap_or_default();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let span = tracing::debug_span!(
        "request",
        method = %method,
        path = path,
        trace_id = trace_id,
    );
    let _guard = span.enter();
    next.run(request).await
}

async fn get_index(Extension(context): Extension<Context>) -> AxumJson<IndexResponse> {
    AxumJson(IndexResponse {
        public_key: context.noise_config().public_key(),
    })
}

async fn get_health() -> AxumJson<HealthResponse> {
    AxumJson(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn fallback_not_found() -> (StatusCode, AxumJson<ErrorResponse>) {
    let code = StatusCode::NOT_FOUND;
    (
        code,
        AxumJson(ErrorResponse::new(code, "Not Found".to_owned())),
    )
}

async fn normalize_axum_error_responses(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    let status = response.status();
    let is_json = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));

    if is_json {
        return response;
    }

    // Route-level axum rejections (e.g. wrong method/body limit) can bypass
    // `ServiceError` conversion and otherwise emit non-JSON default bodies.
    match status {
        StatusCode::METHOD_NOT_ALLOWED => (
            StatusCode::METHOD_NOT_ALLOWED,
            AxumJson(ErrorResponse::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "Method Not Allowed".to_string(),
            )),
        )
            .into_response(),
        StatusCode::PAYLOAD_TOO_LARGE => (
            StatusCode::PAYLOAD_TOO_LARGE,
            AxumJson(ErrorResponse::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Payload Too Large".to_string(),
            )),
        )
            .into_response(),
        _ => response,
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        SERVICE_ERROR_COUNTS
            .with_label_values(&[&format!("{:?}", self.error_code())])
            .inc();
        let status = self.http_status_code();
        let body = AxumJson(ErrorResponse::from(&self));
        (status, body).into_response()
    }
}
