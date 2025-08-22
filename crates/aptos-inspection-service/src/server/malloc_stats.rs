use crate::server::utils::CONTENT_TYPE_TEXT;
use anyhow::Result;
use hyper::{Body, StatusCode};
use std::{
    ffi::CStr,
    os::raw::{c_char, c_void},
};

unsafe extern "C" fn write_cb(buf: *mut c_void, s: *const c_char) {
    let out = &mut *(buf as *mut Vec<u8>);
    let stats_cstr = CStr::from_ptr(s).to_bytes();
    let len = std::cmp::min(out.capacity(), stats_cstr.len());
    out.extend_from_slice(&stats_cstr[0..len]);
}

fn get_jemalloc_stats_string(max_len: usize) -> Result<String> {
    // Refresh cached stats. (Best-effort, ignore errors.)
    let _ = jemalloc_ctl::epoch::advance();

    // We do not want any memory allocation in the callback.
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
            Body::from(stats.as_bytes().to_vec()),
            CONTENT_TYPE_TEXT.into(),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("Failed to get malloc stats: {e}")),
            CONTENT_TYPE_TEXT.into(),
        ),
    }
}
