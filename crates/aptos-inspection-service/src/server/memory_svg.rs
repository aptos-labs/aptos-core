// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_SVG;
use hyper::{Body, StatusCode};
use std::fs;

pub fn handle_memory_svg_request() -> (StatusCode, Body, String) {
    let content =
        fs::read_to_string("./crates/aptos-inspection-service/src/server/memory_profile/heap.svg")
            .expect("Failed to read input");

    (StatusCode::OK, Body::from(content), CONTENT_TYPE_SVG.into())
}
