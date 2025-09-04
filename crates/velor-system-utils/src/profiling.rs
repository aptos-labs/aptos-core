// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::{reply_with, reply_with_status};
use anyhow::{anyhow, ensure};
use async_mutex::Mutex;
use http::header::{HeaderValue, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::{Body, Request, Response, StatusCode};
use lazy_static::lazy_static;
use pprof::protos::Message;
use regex::Regex;
use std::{collections::HashMap, time::Duration};
use tracing::info;

lazy_static! {
    static ref CPU_PROFILE_MUTEX: Mutex<()> = Mutex::new(());
}

pub async fn handle_cpu_profiling_request(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let query = req.uri().query().unwrap_or("");
    let query_pairs: HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes()).collect();

    let seconds: u64 = match query_pairs.get("seconds") {
        Some(val) => match val.parse() {
            Ok(val) => val,
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => 10,
    };

    let frequency: i32 = match query_pairs.get("frequency") {
        Some(val) => match val.parse() {
            Ok(val) => val,
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => 99,
    };

    let use_proto = match query_pairs.get("format") {
        Some(format) => match format.as_ref() {
            "proto" => true,
            "flamegraph" => false,
            _ => {
                return Ok(reply_with_status(
                    StatusCode::BAD_REQUEST,
                    "Unsupported format.",
                ))
            },
        },
        _ => true,
    };

    match start_cpu_profiling(seconds, frequency, use_proto).await {
        Ok(body) => {
            let content_type = if use_proto {
                mime::APPLICATION_OCTET_STREAM
            } else {
                mime::IMAGE_SVG
            };
            let headers: Vec<(_, HeaderValue)> = vec![
                (CONTENT_LENGTH, HeaderValue::from(body.len())),
                (CONTENT_DISPOSITION, HeaderValue::from_static("inline")),
                (
                    CONTENT_TYPE,
                    HeaderValue::from_str(content_type.as_ref()).unwrap(),
                ),
            ];
            Ok(reply_with(headers, body))
        },
        Err(e) => {
            info!("Failed to generate cpu profile: {e:?}");
            Ok(reply_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ))
        },
    }
}

pub async fn start_cpu_profiling(
    seconds: u64,
    frequency: i32,
    use_proto: bool,
) -> anyhow::Result<Vec<u8>> {
    info!(
        seconds = seconds,
        frequency = frequency,
        use_proto = use_proto,
        "Starting cpu profiling."
    );
    let lock = CPU_PROFILE_MUTEX.try_lock();
    ensure!(lock.is_some(), "A profiling task is already running.");

    // TODO(grao): Consolidate the code with velor-profiler crate.
    let guard = pprof::ProfilerGuard::new(frequency)
        .map_err(|e| anyhow!("Failed to start cpu profiling: {e:?}."))?;

    tokio::time::sleep(Duration::from_secs(seconds)).await;

    let mut body = Vec::new();
    let report = guard
        .report()
        .frames_post_processor(frames_post_processor())
        .build()
        .map_err(|e| anyhow!("Failed to generate cpu profiling report: {e:?}."))?;

    if use_proto {
        report
            .pprof()
            .map_err(|e| anyhow!("Failed to generate proto report: {e:?}."))?
            .write_to_vec(&mut body)
            .map_err(|e| anyhow!("Failed to serialize proto report: {e:?}."))?;
    } else {
        report
            .flamegraph(&mut body)
            .map_err(|e| anyhow!("Failed to generate flamegraph report: {e:?}."))?;
    }

    info!("Cpu profiling is done.");

    Ok(body)
}

fn frames_post_processor() -> impl Fn(&mut pprof::Frames) {
    let regex = Regex::new(r"^(.*)-(\d*)$").unwrap();

    move |frames| {
        if let Some((_, [name, _])) = regex.captures(&frames.thread_name).map(|c| c.extract()) {
            frames.thread_name = name.to_string();
        }
    }
}
