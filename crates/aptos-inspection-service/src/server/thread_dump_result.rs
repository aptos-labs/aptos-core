// Copyright © Aptos Foundation

use hyper::{Body, StatusCode};
use crate::{
    server::utils::CONTENT_TYPE_TEXT
};

use std::fs;

pub fn handle_thread_dump_result_request() -> (StatusCode, Body, String) {

    let content = fs::read_to_string("/home/yunusozer/thread_dump.txt").expect("Failed to read input");


    (
        StatusCode::OK,
        Body::from(content),
        CONTENT_TYPE_TEXT.into(),
    )
}
