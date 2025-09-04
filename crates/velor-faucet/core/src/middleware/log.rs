// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::metrics::{HISTOGRAM, RESPONSE_STATUS};
use velor_logger::{
    error, info,
    prelude::{sample, SampleRate},
    warn, Schema,
};
use poem::{
    http::header, web::RealIp, Endpoint, FromRequest, Request, RequestBody, Response, Result,
};
use poem_openapi::OperationId;
use std::{net::IpAddr, time::Duration};

/// Logs information about the request and response. We log at different log
/// levels depending on the response status code / the operation ID. We do both
/// structured logging as well as pushing counters / gauges / etc.
///
/// You'll notice that most of the meat of this logging happens in DropLogger,
/// see the comment there for an explanation of why.
pub async fn middleware_log<E: Endpoint>(next: E, request: Request) -> Result<Response> {
    let start = std::time::Instant::now();

    let source_ip = RealIp::from_request(&request, &mut RequestBody::default())
        .await
        .map(|ip| ip.0)
        .unwrap_or(None);

    let request_log = HttpRequestLog {
        source_ip,
        method: request.method().to_string(),
        path: request.uri().path().to_string(),
        referer: request
            .headers()
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
        user_agent: request
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
        forwarded: request
            .headers()
            .get(header::FORWARDED)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
    };

    let mut drop_logger = DropLogger::new(request_log);

    let response = next.get_response(request).await;

    let elapsed = start.elapsed();
    let response_status = response.status().as_u16();
    let operation_id = response
        .data::<OperationId>()
        .map(|operation_id| operation_id.0)
        .unwrap_or("operation_id_not_set");

    drop_logger.attach_response_log(HttpResponseLog {
        response_status,
        operation_id,
        elapsed,
    });

    Ok(response)
}

// TODO: Figure out how to have certain fields be borrowed, like in the
// original implementation.
/// HTTP request log, keeping track of the requests
#[derive(Schema)]
pub struct HttpRequestLog {
    #[schema(display)]
    source_ip: Option<IpAddr>,
    method: String,
    path: String,
    referer: Option<String>,
    user_agent: Option<String>,
    forwarded: Option<String>,
}

#[derive(Schema)]
pub struct HttpResponseLog<'a> {
    operation_id: &'a str,
    response_status: u16,
    #[schema(debug)]
    elapsed: std::time::Duration,
}

#[derive(Schema)]
pub struct ProcessInfo {
    pod_name: Option<String>,
}

/// In Poem, if the client hangs up mid request, the future stops getting polled
/// and instead gets dropped. So if we want this middleware logging to happen
/// even if this happens, we have to implement the logging in a Drop impl. If
/// we reach this drop impl and there is no response log attached, we have hit
/// this case and log accordingly.
pub struct DropLogger<'a> {
    request_log: HttpRequestLog,
    response_log: Option<HttpResponseLog<'a>>,
}

impl<'a> DropLogger<'a> {
    pub fn new(request_log: HttpRequestLog) -> Self {
        Self {
            request_log,
            response_log: None,
        }
    }

    pub fn attach_response_log(&mut self, response_log: HttpResponseLog<'a>) {
        self.response_log = Some(response_log);
    }
}

impl Drop for DropLogger<'_> {
    fn drop(&mut self) {
        // Get some process info, e.g. the POD_NAME in case we're in a k8s context.
        let process_info = ProcessInfo {
            pod_name: std::env::var("POD_NAME").ok(),
        };

        match &self.response_log {
            Some(response_log) => {
                // Log response statuses generally.
                RESPONSE_STATUS
                    .with_label_values(&[response_log.response_status.to_string().as_str()])
                    .observe(response_log.elapsed.as_secs_f64());

                // Log response status per-endpoint + method.
                HISTOGRAM
                    .with_label_values(&[
                        self.request_log.method.as_str(),
                        response_log.operation_id,
                        response_log.response_status.to_string().as_str(),
                    ])
                    .observe(response_log.elapsed.as_secs_f64());

                // For now log all requests, no sampling, unless it is for `/`.
                if response_log.operation_id == "root" {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(60)),
                        info!(self.request_log, *response_log, process_info)
                    );
                } else if response_log.response_status >= 500 {
                    error!(self.request_log, *response_log, process_info);
                } else {
                    info!(self.request_log, *response_log, process_info);
                }
            },
            None => {
                // If we don't have a response log, it means the client
                // hung up mid-request.
                warn!(self.request_log, process_info, destiny = "hangup");
            },
        }
    }
}
