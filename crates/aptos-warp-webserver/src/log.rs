// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use axum::{
    body::Body,
    extract::MatchedPath,
    http::{header, Request},
    middleware::{from_fn, Next},
    response::Response,
    Router,
};
use aptos_logger::prelude::{sample, SampleRate};
use std::time::{Duration, Instant};

pub fn logger<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(from_fn(log_middleware))
}

async fn log_middleware(request: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_owned())
        .unwrap_or_else(|| request.uri().path().to_owned());
    let forwarded = request
        .headers()
        .get(header::FORWARDED)
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);
    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);
    let referer = request
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);

    let response = next.run(request).await;
    let status = response.status().as_u16();
    let elapsed = start.elapsed();

    if status >= 500 {
        sample!(
            SampleRate::Duration(Duration::from_secs(1)),
            aptos_logger::error!(
                method = method,
                path = path,
                status = status,
                elapsed = ?elapsed,
                forwarded = forwarded,
                user_agent = user_agent,
                referer = referer,
                "http request failed"
            )
        );
    } else {
        aptos_logger::debug!(
            method = method,
            path = path,
            status = status,
            elapsed = ?elapsed,
            forwarded = forwarded,
            user_agent = user_agent,
            referer = referer,
            "http request handled"
        );
    }

    response
}
