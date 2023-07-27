// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_HTML;
use hyper::{Body, StatusCode};
use std::{fs, fs::File, io::Read, thread, time};
use std::process::Command;
use std::time::Duration;
use crate::utils::{CONTENT_TYPE_JSON, CONTENT_TYPE_SVG, CONTENT_TYPE_TEXT};
use aptos_profiler::{ProfilerHandler, ProfilerConfig};

pub fn handle_profiling_request() -> (StatusCode, Body, String) {
    let mut file = File::open("./crates/aptos-inspection-service/src/server/index.html")
        .expect("Failed to open file");

    // Read the contents of the file into a string
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");

    (StatusCode::OK, Body::from(contents), CONTENT_TYPE_HTML.into(),)
}

pub fn handle_cpu_profiling_request() -> (StatusCode, Body, String) {
    //Call aptos-profiling cpu profiling 
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let cpu_profiler = handler.get_cpu_profiler();
    cpu_profiler.start_profiling();

    (StatusCode::OK, Body::from("Success"), CONTENT_TYPE_JSON.into(),)
}

pub fn handle_memory_profiling_request() -> (StatusCode, Body, String) {
    //Call aptos-profiling memory profiling 
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let memory_profiler = handler.get_mem_profiler();
    memory_profiler.start_profiling();

    (StatusCode::OK, Body::from("Success"), CONTENT_TYPE_TEXT.into())
}

//TODO: use aptos-profiler crater
pub fn handle_cpu_flamegraph_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let cpu_profiler = handler.get_cpu_profiler();
    let result = cpu_profiler.expose_svg_results();

    (StatusCode::OK, Body::from(result.unwrap()), CONTENT_TYPE_SVG.into())
}

//TODO: use aptos-profiler crater
pub fn handle_memory_svg_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let memory_profiler = handler.get_mem_profiler();
    let result = memory_profiler.expose_svg_results();
    
    (StatusCode::OK, Body::from(result.unwrap()), CONTENT_TYPE_SVG.into())
}

//TODO: use aptos-profiler crater
pub fn handle_memory_txt_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let memory_profiler = handler.get_mem_profiler();
    let result = memory_profiler.expose_text_results();
    
    (StatusCode::OK, Body::from(result.unwrap()), CONTENT_TYPE_TEXT.into())
}

pub fn handle_thread_dump_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let thread_profiler = handler.get_thread_profiler();
    thread_profiler.start_profiling();

    (StatusCode::OK, Body::from("555"), CONTENT_TYPE_TEXT.into())
}

//TODO: use aptos-profiler crater
pub fn handle_thread_dump_result_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let thread_profiler = handler.get_thread_profiler();
    let result = thread_profiler.expose_text_results();
    
    (StatusCode::OK, Body::from(result.unwrap()), CONTENT_TYPE_TEXT.into())
}
