// Copyright © Aptos Foundation

use hyper::{Body, StatusCode};
use crate::{
    server::utils::CONTENT_TYPE_SVG
};

use std::fs;

pub fn handle_cpu_flamegraph_request() -> (StatusCode, Body, String) {

    let content = fs::read_to_string("/home/yunusozer/aptos-core/crates/aptos-inspection-service/src/server/profiling_dashboard/flamegraph.svg").expect("Failed to read input");


    (
        StatusCode::OK,
        Body::from(content),
        CONTENT_TYPE_SVG.into(),
    )
}
