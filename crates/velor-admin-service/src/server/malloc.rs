// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_system_utils::utils::{reply_with, reply_with_status};
use hyper::{Body, Response, StatusCode};
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_void},
    time::SystemTime,
};

const PROFILE_PATH_PREFIX: &str = "/tmp/heap-profile";

unsafe extern "C" fn write_cb(buf: *mut c_void, s: *const c_char) {
    let out = &mut *(buf as *mut Vec<u8>);
    let stats_cstr = CStr::from_ptr(s).to_bytes();
    // We do not want any memory allocation in the callback.
    let len = std::cmp::min(out.capacity(), stats_cstr.len());
    out.extend_from_slice(&stats_cstr[0..len]);
}

fn get_jemalloc_stats_string(max_len: usize) -> anyhow::Result<String> {
    let _ = jemalloc_ctl::epoch::advance();

    let mut stats = Vec::with_capacity(max_len);
    unsafe {
        jemalloc_sys::malloc_stats_print(
            Some(write_cb),
            &mut stats as *mut _ as *mut c_void,
            std::ptr::null(),
        );
    }
    Ok(String::from_utf8(stats)?)
}

pub fn handle_malloc_stats_request(max_len: usize) -> hyper::Result<Response<Body>> {
    match get_jemalloc_stats_string(max_len) {
        Ok(stats) => Ok(reply_with(Vec::new(), Body::from(stats))),
        Err(e) => Ok(reply_with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get malloc stats: {e}"),
        )),
    }
}

fn dump_heap_profile() -> anyhow::Result<String> {
    let _ = jemalloc_ctl::epoch::advance();

    let key = b"prof.dump\0";
    let path = format!(
        "{}.{}",
        PROFILE_PATH_PREFIX,
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis()
    );
    let value = CString::new(path.clone())?;
    unsafe {
        jemalloc_ctl::raw::write(key, value.as_ptr())
            .map_err(|e| anyhow::anyhow!("prof.dump error: {e}"))?;
    }
    Ok(path)
}

pub fn handle_dump_profile_request() -> hyper::Result<Response<Body>> {
    match dump_heap_profile() {
        Ok(path) => Ok(reply_with(
            Vec::new(),
            Body::from(format!("Successfully dumped heap profile to {path}")),
        )),
        Err(e) => Ok(reply_with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to dump heap profile: {e}"),
        )),
    }
}
