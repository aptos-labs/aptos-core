use crate::server::utils::CONTENT_TYPE_TEXT;
use anyhow::Result;
use hyper::{Body, StatusCode};
use jemalloc_sys as je;
use std::{
    ffi::CStr,
    os::raw::{c_char, c_void},
};

unsafe extern "C" fn cap_cb(buf: *mut c_void, s: *const c_char) {
    let out = &mut *(buf as *mut Vec<u8>);
    let stats_cstr = CStr::from_ptr(s).to_bytes();
    // Make sure we do not allocate any memory here.
    let len = out.capacity().min(stats_cstr.len());
    out.extend_from_slice(&stats_cstr[0..len]);
}

fn get_jemalloc_stats_string(max_len: usize) -> Result<String> {
    // Refresh cached stats. (Best-effort, ignore errors.)
    let _ = jemalloc_ctl::epoch::advance();

    // We do not want any memory allocation in the callback.
    let mut stats = Vec::with_capacity(max_len);
    unsafe {
        je::malloc_stats_print(
            Some(cap_cb),
            &mut stats as *mut _ as *mut c_void,
            b"\0".as_ptr() as *const c_char,
        );
    }
    Ok(String::from_utf8(stats)?)
}

pub fn handle_malloc_stats_request(max_len: usize) -> (StatusCode, Body, String) {
    match get_jemalloc_stats_string(max_len) {
        Ok(stats) => {
            let buffer = stats.as_bytes().to_vec();
            (StatusCode::OK, Body::from(buffer), CONTENT_TYPE_TEXT.into())
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("{e}")),
            CONTENT_TYPE_TEXT.into(),
        ),
    }
}
