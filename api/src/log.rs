// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::metrics::{HISTOGRAM, RESPONSE_STATUS};
use aptos_logger::{
    debug, error, info,
    prelude::{sample, SampleRate},
    sample::Sampling,
    Schema,
};
use poem::{http::header, Endpoint, Request, Response, Result};
use poem_openapi::OperationId;

/// Logs information about the request and response if the response status code
/// is >= 500, to help us debug since this will be an error on our side.
/// We also do general logging of the status code alone regardless of what it is.
pub async fn middleware_log<E: Endpoint>(next: E, request: Request) -> Result<Response> {
    let start = std::time::Instant::now();

    let mut log = HttpRequestLog {
        remote_addr: request.remote_addr().as_socket_addr().cloned(),
        method: request.method().to_string(),
        path: request.uri().path().to_string(),
        status: 0,
        referer: request
            .headers()
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
        user_agent: request
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
        elapsed: Duration::from_secs(0),
        forwarded: request
            .headers()
            .get(header::FORWARDED)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
    };

    let response = next.get_response(request).await;

    let elapsed = start.elapsed();

    log.status = response.status().as_u16();
    log.elapsed = elapsed;

    if log.status >= 500 {
        sample!(SampleRate::Duration(Duration::from_secs(1)), error!(log));
    } else if log.status >= 400 {
        sample!(SampleRate::Duration(Duration::from_secs(60)), info!(log));
    } else {
        sample!(SampleRate::Duration(Duration::from_secs(1)), debug!(log));
    }

    // Log response statuses generally.
    RESPONSE_STATUS
        .with_label_values(&[log.status.to_string().as_str()])
        .observe(elapsed.as_secs_f64());

    // Log response status per-endpoint + method.
    HISTOGRAM
        .with_label_values(&[
            log.method.as_str(),
            response
                .data::<OperationId>()
                .map(|operation_id| operation_id.0)
                .unwrap_or("operation_id_not_set"),
            log.status.to_string().as_str(),
        ])
        .observe(elapsed.as_secs_f64());

    Ok(response)
}

// TODO: Figure out how to have certain fields be borrowed, like in the
// original implementation.
/// HTTP request log, keeping track of the requests
#[derive(Schema)]
pub struct HttpRequestLog {
    #[schema(display)]
    remote_addr: Option<std::net::SocketAddr>,
    method: String,
    path: String,
    pub status: u16,
    referer: Option<String>,
    user_agent: Option<String>,
    #[schema(debug)]
    pub elapsed: std::time::Duration,
    forwarded: Option<String>,
}
