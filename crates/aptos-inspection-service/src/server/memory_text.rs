// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_TEXT;
use hyper::{Body, StatusCode};
use std::fs;

pub fn handle_memory_txt_request() -> (StatusCode, Body, String) {
    let content =
        fs::read_to_string("./crates/aptos-inspection-service/src/server/memory_profile/heap.txt")
            .expect("Failed to read input");

    (
        StatusCode::OK,
        Body::from(content),
        CONTENT_TYPE_TEXT.into(),
    )
}
