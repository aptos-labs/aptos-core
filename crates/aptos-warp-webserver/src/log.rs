// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use aptos_logger::{
    debug, error,
    prelude::{sample, SampleRate},
    sample::Sampling,
    Schema,
};
use warp::{
    http::header,
    log::{custom, Info, Log},
};

pub fn logger() -> Log<impl Fn(Info) + Copy> {
    let func = move |info: Info| {
        let status = info.status().as_u16();
        let log = HttpRequestLog {
            remote_addr: info.remote_addr(),
            method: info.method().to_string(),
            path: info.path().to_string(),
            status,
            referer: info.referer(),
            user_agent: info.user_agent(),
            elapsed: info.elapsed(),
            forwarded: info
                .request_headers()
                .get(header::FORWARDED)
                .and_then(|v| v.to_str().ok()),
        };
        if status >= 500 {
            sample!(SampleRate::Duration(Duration::from_secs(1)), error!(log));
        } else {
            debug!(log);
        }
    };
    custom(func)
}

#[derive(Schema)]
pub struct HttpRequestLog<'a> {
    #[schema(display)]
    remote_addr: Option<std::net::SocketAddr>,
    method: String,
    path: String,
    status: u16,
    referer: Option<&'a str>,
    user_agent: Option<&'a str>,
    #[schema(debug)]
    elapsed: std::time::Duration,
    forwarded: Option<&'a str>,
}
