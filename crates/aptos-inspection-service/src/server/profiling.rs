// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_HTML;
use hyper::{Body, StatusCode};
use std::{fs, fs::File, io::Read, thread, time};
use std::process::Command;
use std::time::Duration;
use crate::utils::{CONTENT_TYPE_JSON, CONTENT_TYPE_SVG, CONTENT_TYPE_TEXT};
use aptos_profiler::{ProfilerHandler, ProfilerConfig};

#[cfg(target_os = "linux")]
pub fn handle_profiling_request() -> (StatusCode, Body, String) {
    let mut file = File::open("./crates/aptos-inspection-service/src/server/index.html")
        .expect("Failed to open file");

    // Read the contents of the file into a string
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");

    (StatusCode::OK, Body::from(contents), CONTENT_TYPE_HTML.into(),)
}

#[cfg(target_os = "linux")]
pub fn handle_cpu_profiling_request() -> (StatusCode, Body, String) {
    // Call aptos-profiling cpu profiling
    //let config = ProfilerConfig::load_from_file(PathBuf::from("./config.yml"));
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let cpu_profiler = handler.get_cpu_profiler();

    match cpu_profiler.start_profiling() {
        Ok(_) => {
            // If profiling started successfully, return the OK status code
            (StatusCode::OK, Body::from("Success"), CONTENT_TYPE_TEXT.into())
        }
        Err(_) => {
            // If an error occurred during profiling start, return a different status code
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error starting CPU profiling"), CONTENT_TYPE_TEXT.into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn handle_memory_profiling_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let memory_profiler = handler.get_mem_profiler();

    match memory_profiler.start_profiling() {
        Ok(_) => {
            (StatusCode::OK, Body::from("Success"), CONTENT_TYPE_TEXT.into())
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error starting memory profiling"), CONTENT_TYPE_TEXT.into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn handle_cpu_flamegraph_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let cpu_profiler = handler.get_cpu_profiler();
    let result = cpu_profiler.expose_svg_results();

    match result {
        Ok(svg_data) => {
            (StatusCode::OK, Body::from(svg_data), CONTENT_TYPE_SVG.into())
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error generating CPU flamegraph"), CONTENT_TYPE_TEXT.into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn handle_memory_svg_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let memory_profiler = handler.get_mem_profiler();
    let result = memory_profiler.expose_svg_results();

    match result {
        Ok(svg_data) => {
            (StatusCode::OK, Body::from(svg_data), CONTENT_TYPE_SVG.into())
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error generating memory SVG"), CONTENT_TYPE_TEXT.into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn handle_memory_txt_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let memory_profiler = handler.get_mem_profiler();
    let result = memory_profiler.expose_text_results();

    match result {
        Ok(text_data) => {
            (StatusCode::OK, Body::from(text_data), CONTENT_TYPE_TEXT.into())
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error generating memory text results"), CONTENT_TYPE_TEXT.into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn handle_thread_dump_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let thread_profiler = handler.get_thread_profiler();

    match thread_profiler.start_profiling() {
        Ok(_) => {
            (StatusCode::OK, Body::from("555"), CONTENT_TYPE_TEXT.into())
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error starting thread profiling"), CONTENT_TYPE_TEXT.into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn handle_thread_dump_result_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let thread_profiler = handler.get_thread_profiler();
    let result = thread_profiler.expose_text_results();

    match result {
        Ok(text_data) => {
            (StatusCode::OK, Body::from(text_data), CONTENT_TYPE_TEXT.into())
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error generating thread dump results"), CONTENT_TYPE_TEXT.into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn handle_offcpu_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let offcpu_profiler = handler.get_offcpu_profiler();

    match offcpu_profiler.start_profiling() {
        Ok(_) => {
            (StatusCode::OK, Body::from("Success"), CONTENT_TYPE_TEXT.into())
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error starting offcpu profiling"), CONTENT_TYPE_TEXT.into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn handle_offcpu_result_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let offcpu_profiler = handler.get_offcpu_profiler();
    let result = offcpu_profiler.expose_svg_results();

    match result {
        Ok(svg_data) => {
            (StatusCode::OK, Body::from(svg_data), CONTENT_TYPE_SVG.into())
        }
        Err(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Body::from("Error generating offcpu profiling results"), CONTENT_TYPE_TEXT.into())
        }
    }
}
