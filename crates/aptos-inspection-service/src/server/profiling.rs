// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_HTML;
use hyper::{Body, StatusCode};
use std::{fs, fs::File, io::Read, thread, time};
use std::process::Command;
use std::time::Duration;
use crate::utils::{CONTENT_TYPE_JSON, CONTENT_TYPE_SVG, CONTENT_TYPE_TEXT};
use crate::aptos_profiler::{ProfilerHandler, ProfilerConfig};

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

//TODO: use aptos-profiler crater
pub fn handle_cpu_profiling_request() -> (StatusCode, Body, String) {
    let guard = pprof::ProfilerGuard::new(100).unwrap();
    let five_secs = time::Duration::from_millis(5000);
    thread::sleep(five_secs);

    if let Ok(report) = guard.report().build() {
        let file = File::create("./flamegraph.svg").unwrap();
        report.flamegraph(file).unwrap();

        println!("report: {:?}", &report);
    };

    (
        StatusCode::OK,
        Body::from("{\"id\": 12020}"),
        CONTENT_TYPE_JSON.into(),
    )
}

//TODO: use aptos-profiler crater
pub fn handle_memory_profiling_request() -> (StatusCode, Body, String) {
    unsafe {
        let mut prof_active: bool = true;

        let result = jemalloc_sys::mallctl(
            b"prof.active\0".as_ptr() as *const _,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut prof_active as *mut _ as *mut _,
            std::mem::size_of::<bool>(),
        );

        println!("{}", result);
        if result != 0 {
            panic!("Failed to activate jemalloc profiling");
        }

        let handle = thread::spawn(move || {
            // Sleep for 15 seconds
            thread::sleep(Duration::from_secs(120));

            // Disable the profiling
            let mut prof_active: bool = false;
            let result = jemalloc_sys::mallctl(
                b"prof.active\0".as_ptr() as *const _,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut prof_active as *mut _ as *mut _,
                std::mem::size_of::<bool>(),
            );

            println!("{}", result);
            if result != 0 {
                panic!("Failed to deactivate jemalloc profiling");
            }
        });

        handle.join().unwrap();
    }
    let output = Command::new("python3")
        .arg("/home/yunusozer/aptos-core/crates/aptos-inspection-service/src/server/memory_profile/jeprof.py")
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Command executed successfully. Output:\n{}", stdout);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("Command failed. Error:\n{}", stderr);
    }

    (StatusCode::OK, Body::from("555"), CONTENT_TYPE_TEXT.into())
}

//TODO: use aptos-profiler crater
pub fn handle_cpu_flamegraph_request() -> (StatusCode, Body, String) {
    let content = fs::read_to_string("./flamegraph.svg").expect("Failed to read input");

    (StatusCode::OK, Body::from(content), CONTENT_TYPE_SVG.into())
}

//TODO: use aptos-profiler crater
pub fn handle_memory_svg_request() -> (StatusCode, Body, String) {
    let content =
        fs::read_to_string("./crates/aptos-inspection-service/src/server/memory_profile/heap.svg")
            .expect("Failed to read input");

    (StatusCode::OK, Body::from(content), CONTENT_TYPE_SVG.into())
}

//TODO: use aptos-profiler crater
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

pub fn handle_thread_dump_request() -> (StatusCode, Body, String) {
    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);
    let thread_profiler = handler.get_thread_profiler();
    thread_profiler.start_profiling();

    (StatusCode::OK, Body::from("555"), CONTENT_TYPE_TEXT.into())
}

//TODO: use aptos-profiler crater
pub fn handle_thread_dump_result_request() -> (StatusCode, Body, String) {
    let content = fs::read_to_string("./thread_dump.txt").expect("Failed to read input");

    (
        StatusCode::OK,
        Body::from(content),
        CONTENT_TYPE_TEXT.into(),
    )
}
