use hyper::{Body, StatusCode};
use crate::{
    server::utils::CONTENT_TYPE_SVG, CONFIGURATION_PATH, FORGE_METRICS_PATH, JSON_METRICS_PATH,
    METRICS_PATH, PEER_INFORMATION_PATH, SYSTEM_INFORMATION_PATH,
};
use std::{thread, time};

use std::fs;
use std::fs::File;

use std::io::Read;

pub fn handle_cpu_flamegraph_request() -> (StatusCode, Body, String) {
    
    let content = fs::read_to_string("/home/ubuntu/aptos-core/crates/aptos-inspection-service/src/server/profiling_dashboard/flamegraph.svg").expect("Failed to read input");

    
    (
        StatusCode::OK,
        Body::from(content),
        CONTENT_TYPE_SVG.into(),
    )
}