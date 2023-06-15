use hyper::{Body, StatusCode};
use crate::{
    server::utils::CONTENT_TYPE_SVG
};

use std::fs;

pub fn handle_memory_svg_request() -> (StatusCode, Body, String) {
    
    let content = fs::read_to_string("/home/ubuntu/aptos-core/crates/aptos-inspection-service/src/server/memory_profile/heap.svg").expect("Failed to read input");

    
    (
        StatusCode::OK,
        Body::from(content),
        CONTENT_TYPE_SVG.into(),
    )
}