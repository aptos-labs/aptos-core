use crate::server::utils::CONTENT_TYPE_TEXT;
use anyhow::{anyhow, Result};
use aptos_logger::prelude::*;
use hyper::{Body, StatusCode};
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
    if len < stats_cstr.len() {
        warn!(
            "Length of malloc stats {} larger than pre-allocated length {}. Truncating...",
            stats_cstr.len(),
            out.capacity()
        );
    }
    out.extend_from_slice(&stats_cstr[0..len]);
}

fn get_jemalloc_stats_string(max_len: usize) -> Result<String> {
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

pub fn handle_malloc_stats_request(max_len: usize) -> (StatusCode, Body, String) {
    match get_jemalloc_stats_string(max_len) {
        Ok(stats) => (
            StatusCode::OK,
            Body::from(stats.into_bytes()),
            CONTENT_TYPE_TEXT.into(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("Failed to get malloc stats: {e}")),
            CONTENT_TYPE_TEXT.into(),
        ),
    }
}

fn dump_heap_profile() -> Result<()> {
    let _ = jemalloc_ctl::epoch::advance();

    let key = b"prof.dump\0";
    let path = format!(
        "{}.{}",
        PROFILE_PATH_PREFIX,
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis()
    );
    let value = CString::new(path)?;
    unsafe {
        jemalloc_ctl::raw::write::<*const c_char>(key, std::ptr::null())
            .map_err(|e| anyhow!("prof.dump error: {e}"))?;
    }
    Ok(())
}

pub fn handle_dump_profile_request() -> (StatusCode, Body, String) {
    match dump_heap_profile() {
        Ok(()) => (
            StatusCode::OK,
            Body::from("Successfully dumped heap profile"),
            CONTENT_TYPE_TEXT.into(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("Failed to dump heap profile: {e}")),
            CONTENT_TYPE_TEXT.into(),
        ),
    }
}
