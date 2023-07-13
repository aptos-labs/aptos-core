// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_SVG;
use hyper::{Body, StatusCode};
use std::fs;

pub fn handle_cpu_flamegraph_request() -> (StatusCode, Body, String) {
    let content = fs::read_to_string("./flamegraph.svg").expect("Failed to read input");

    (StatusCode::OK, Body::from(content), CONTENT_TYPE_SVG.into())
}
