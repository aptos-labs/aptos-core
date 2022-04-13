// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::{aptos_logger::AptosData, debug, error, trace, Level};
use serde::Deserialize;
use std::{
    io::{BufRead, BufReader},
    net::TcpListener,
};

#[derive(Deserialize)]
struct Log {
    level: Level,
    backtrace: Option<String>,
}

#[test]
fn remote_end_to_end() {
    std::env::set_var("RUST_LOG_REMOTE", "debug");

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    AptosData::builder()
        .address(addr)
        .is_async(true)
        .enable_backtrace()
        .build();

    let handle = std::thread::spawn(|| {
        error!("Hello");
        trace!("Hello");
        debug!("Hello");
        aptos_logger::flush();
    });

    let (stream, _) = listener.accept().unwrap();

    let mut stream = BufReader::new(stream);
    let mut buf = Vec::new();
    stream.read_until(b'\n', &mut buf).unwrap();

    let log: Log = serde_json::from_slice(&buf).unwrap();
    assert!(log.backtrace.is_some());
    assert_eq!(log.level, Level::Error);

    let mut buf = Vec::new();
    stream.read_until(b'\n', &mut buf).unwrap();

    let log: Log = serde_json::from_slice(&buf).unwrap();
    assert_eq!(log.level, Level::Debug);

    // Test that flush properly returns
    handle.join().unwrap();
}
