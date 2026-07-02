// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_logger::{
    debug, error,
    prelude::{sample, SampleRate},
    Schema,
};
use axum::{
    extract::Request,
    http::{header, HeaderMap, HeaderName},
    middleware::Next,
    response::Response,
};
use std::time::Duration;

/// Axum middleware that logs each request via `aptos_logger`, sampling 5xx
/// responses. Mirrors the behavior of `aptos_warp_webserver::logger`.
///
/// Wrap it at the call site with `axum::middleware::from_fn`:
/// `.layer(axum::middleware::from_fn(aptos_axum_webserver::log_middleware))`.
///
/// Note: unlike the warp logger, `remote_addr` is not captured (it requires
/// `ConnectInfo` wiring) and the structured log format is not byte-identical.
pub async fn log_middleware(req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let referer = header_string(req.headers(), header::REFERER);
    let user_agent = header_string(req.headers(), header::USER_AGENT);
    let forwarded = header_string(req.headers(), header::FORWARDED);

    let start = tokio::time::Instant::now();
    let response = next.run(req).await;
    let elapsed = start.elapsed();

    let status = response.status().as_u16();
    let log = HttpRequestLog {
        remote_addr: None,
        method,
        path,
        status,
        referer,
        user_agent,
        elapsed,
        forwarded,
    };
    if status >= 500 {
        sample!(SampleRate::Duration(Duration::from_secs(1)), error!(log));
    } else {
        debug!(log);
    }
    response
}

fn header_string(headers: &HeaderMap, name: HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
}

#[derive(Schema)]
pub struct HttpRequestLog {
    #[schema(display)]
    remote_addr: Option<std::net::SocketAddr>,
    method: String,
    path: String,
    status: u16,
    referer: Option<String>,
    user_agent: Option<String>,
    #[schema(debug)]
    elapsed: std::time::Duration,
    forwarded: Option<String>,
}
