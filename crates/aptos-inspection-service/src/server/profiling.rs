use hyper::{Body, StatusCode};
use crate::{
    server::utils::{CONTENT_TYPE_TEXT, CONTENT_TYPE_HTML}, CONFIGURATION_PATH, FORGE_METRICS_PATH, JSON_METRICS_PATH,
    METRICS_PATH, PEER_INFORMATION_PATH, SYSTEM_INFORMATION_PATH,
};
use std::{thread, time};

use std::fs::File;
use std::io::Read;

pub fn handle_profiling_request() -> (StatusCode, Body, String) {
    let mut file = File::open("/home/ubuntu/aptos-core/crates/aptos-inspection-service/src/server/profiling_dashboard/index.html").expect("Failed to open file");

    // Read the contents of the file into a string
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read file");
    (
        StatusCode::OK,
        Body::from(contents),
        CONTENT_TYPE_HTML.into(),
    )
}