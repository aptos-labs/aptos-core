// Copyright © Aptos Foundation

use crate::server::utils::CONTENT_TYPE_TEXT;
use hyper::{Body, StatusCode};
use std::{path::Path, process::Command, thread, time::Duration};

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
