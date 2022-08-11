// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::metrics::RESPONSE_STATUS;
use aptos_logger::{
    debug, error,
    prelude::{sample, SampleRate},
    sample::Sampling,
    Schema,
};
use poem::{http::header, Endpoint, Request, Response, Result};

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
    } else {
        debug!(log);
    }

    // Log response statuses generally.
    RESPONSE_STATUS
        .with_label_values(&[log.status.to_string().as_str()])
        .observe(elapsed.as_secs_f64());

    Ok(response)
}

// TODO: Figure out how to have certain fields be borrowed, like in the
// original implementation.
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

// This macro helps generate a function that can be used to transform an
// endpoint such that it does per-endpoint logging based on operation_id.
// Unfortunately we have to do it this way right now because Poem doesn't
// support accessing operation_id directly in middleware. See this issue
// for more information: https://github.com/poem-web/poem/issues/351.
#[macro_export]
macro_rules! generate_endpoint_logging_functions {
    ($($operation_id:ident),*) => {
        paste::paste! {
        $(
        fn [< $operation_id _log >](ep: impl poem::Endpoint + 'static) -> impl poem::Endpoint + 'static {
            poem::EndpointExt::around(ep, |ep, request| async move {
                let method = request.method().to_string();

                let start = std::time::Instant::now();
                let response = ep.get_response(request).await;
                let elapsed = start.elapsed();

                $crate::metrics::HISTOGRAM
                    .with_label_values(&[
                        method.as_str(),
                        stringify!($operation_id),
                        response.status().as_u16().to_string().as_str(),
                    ])
                    .observe(elapsed.as_secs_f64());

                Ok(response)
            })
        }
        )*
        }
    };
}
