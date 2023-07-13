// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_HTML;
use hyper::{Body, StatusCode};
use std::{fs::File, io::Read};

pub fn handle_profiling_request() -> (StatusCode, Body, String) {
    let mut file = File::open("./crates/aptos-inspection-service/src/server/index.html")
        .expect("Failed to open file");

    // Read the contents of the file into a string
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");
    (
        StatusCode::OK,
        Body::from(contents),
        CONTENT_TYPE_HTML.into(),
    )
}
