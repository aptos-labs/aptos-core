// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::{reply_with, reply_with_status};
use anyhow::{ensure, Error};
use aptos_logger::info;
use async_mutex::Mutex;
use http::header::{HeaderValue, CONTENT_LENGTH};
use hyper::{Body, Request, Response, StatusCode};
use lazy_static::lazy_static;
use rstack_self::TraceOptions;
use std::{collections::HashMap, env, process::Command};

lazy_static! {
    static ref THREAD_DUMP_MUTEX: Mutex<()> = Mutex::new(());
}

pub async fn handle_thread_dump_request(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let query = req.uri().query().unwrap_or("");
    let query_pairs: HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes()).collect();

    let snapshot: bool = match query_pairs.get("snapshot") {
        Some(val) => match val.parse() {
            Ok(val) => val,
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => false,
    };

    let location: bool = match query_pairs.get("location") {
        Some(val) => match val.parse() {
            Ok(val) => val,
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => true,
    };

    let frame_ip: bool = match query_pairs.get("frame_ip") {
        Some(val) => match val.parse() {
            Ok(val) => val,
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => false,
    };

    info!("Starting dumping stack trace for all threads.");
    match start_thread_dump(snapshot, location, frame_ip).await {
        Ok(body) => {
            info!("Thread dumping is done.");
            let headers: Vec<(_, HeaderValue)> =
                vec![(CONTENT_LENGTH, HeaderValue::from(body.len()))];
            Ok(reply_with(headers, body))
        },
        Err(e) => {
            info!("Failed to dump threads: {e:?}");
            Ok(reply_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
            ))
        },
    }
}

async fn start_thread_dump(
    snapshot: bool,
    location: bool,
    frame_ip: bool,
) -> anyhow::Result<String> {
    let lock = THREAD_DUMP_MUTEX.try_lock();
    ensure!(lock.is_some(), "A thread dumping task is already running.");

    let exe = env::current_exe().unwrap();
    let trace = TraceOptions::new()
        .snapshot(snapshot)
        .trace(Command::new(exe).arg("--stacktrace"))
        .map_err(Error::msg)?;

    let mut body = String::new();
    for thread in trace.threads() {
        body.push_str(&format!("Thread {} ({}):\n", thread.id(), thread.name()));
        for frame in thread.frames() {
            if frame_ip {
                body.push_str(&format!("Frame ip: {}\n", frame.ip()));
            }
            for symbol in frame.symbols() {
                let name = if let Some(name) = symbol.name() {
                    name
                } else {
                    "(unknown)"
                };
                if location {
                    let location = if let Some(file) = symbol.file() {
                        if let Some(line) = symbol.line() {
                            format!("{}:{line}", file.display())
                        } else {
                            format!("{}", file.display())
                        }
                    } else {
                        "".into()
                    };
                    body.push_str(&format!("{name}\t\t{location}\n"));
                } else {
                    body.push_str(&format!("{name}\n"));
                }
            }
        }
        body.push_str("\n\n");
    }

    Ok(body)
}
