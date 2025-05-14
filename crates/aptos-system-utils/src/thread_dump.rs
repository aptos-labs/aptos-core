// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::{reply_with, reply_with_status};
use anyhow::{ensure, Error};
use async_mutex::Mutex;
use http::header::{HeaderValue, CONTENT_LENGTH};
use hyper::{Body, Request, Response, StatusCode};
use lazy_static::lazy_static;
use rstack_self::TraceOptions;
use std::{collections::HashMap, env, process::Command};
use tracing::info;

lazy_static! {
    static ref THREAD_DUMP_MUTEX: Mutex<()> = Mutex::new(());
}

static MAX_NUM_FRAMES_WITHOUT_VERBOSE: usize = 20;

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

    let verbose: bool = match query_pairs.get("verbose") {
        Some(val) => match val.parse() {
            Ok(val) => val,
            Err(err) => return Ok(reply_with_status(StatusCode::BAD_REQUEST, err.to_string())),
        },
        None => false,
    };

    info!("Starting dumping stack trace for all threads.");
    match do_thread_dump(snapshot, location, frame_ip, verbose).await {
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

async fn do_thread_dump(
    snapshot: bool,
    location: bool,
    frame_ip: bool,
    verbose: bool,
) -> anyhow::Result<String> {
    let lock = THREAD_DUMP_MUTEX.try_lock();
    ensure!(lock.is_some(), "A thread dumping task is already running.");

    let exe = env::current_exe().unwrap();
    let trace = TraceOptions::new()
        .snapshot(snapshot)
        .trace(Command::new(exe).arg("--stacktrace"))
        .map_err(Error::msg)?;

    let mut wait_threads = Vec::new();
    let mut sleep_threads = Vec::new();
    let mut body = String::new();
    for thread in trace.threads() {
        let frames = thread.frames();
        if !verbose {
            if !frames.is_empty() {
                let symbols = frames[0].symbols();
                if !symbols.is_empty() {
                    if let Some(name) = symbols[0].name() {
                        if name.contains("epoll_wait") {
                            wait_threads.push(thread.name());
                            continue;
                        }

                        if name.contains("clock_nanosleep") {
                            sleep_threads.push(thread.name());
                            continue;
                        }
                    }
                }
            }

            if frames.len() > 1 {
                let symbols = frames[1].symbols();
                if !symbols.is_empty() {
                    if let Some(name) = symbols[0].name() {
                        if name.contains("futex_wait")
                            || name.contains("pthread_cond_wait")
                            || name.contains("pthread_cond_timedwait")
                        {
                            wait_threads.push(thread.name());
                            continue;
                        }
                    }
                }
            }
        }

        body.push_str(&format!("Thread {} ({}):\n", thread.id(), thread.name()));
        for (count, frame) in frames.iter().enumerate() {
            if !verbose && count >= MAX_NUM_FRAMES_WITHOUT_VERBOSE {
                break;
            }

            if frame_ip {
                body.push_str(&format!("Frame ip: {}\n", frame.ip()));
            }
            for symbol in frame.symbols() {
                let name = symbol.name().unwrap_or("(unknown)");
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

    if !verbose {
        body.push_str("Wait threads:");
        for wait_thread in wait_threads {
            body.push_str(&format!(" {wait_thread}"));
        }
        body.push_str("\n\n");

        body.push_str("Sleep threads:");
        for sleep_thread in sleep_threads {
            body.push_str(&format!(" {sleep_thread}"));
        }
        body.push_str("\n\n");
    }

    Ok(body)
}
