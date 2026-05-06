// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_logger::info;
use aptos_system_utils::utils::{reply_with, reply_with_status};
use hyper::{Body, Request, Response, StatusCode};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    os::raw::{c_char, c_void},
    path::Path,
    time::SystemTime,
};

const DEFAULT_PROFILE_PATH_PREFIX: &str = "/tmp/heap-profile";

unsafe extern "C" fn write_cb(buf: *mut c_void, s: *const c_char) {
    let out = unsafe { &mut *(buf as *mut Vec<u8>) };
    let stats_cstr = unsafe { CStr::from_ptr(s).to_bytes() };
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

fn dump_heap_profile(output_path: Option<String>) -> anyhow::Result<String> {
    let _ = jemalloc_ctl::epoch::advance();

    let key = b"prof.dump\0";
    let path = match output_path {
        Some(path) => path,
        None => format!(
            "{}.{}",
            DEFAULT_PROFILE_PATH_PREFIX,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_millis()
        ),
    };
    info!("Dumping heap profile to {path}.");
    let value = CString::new(path.clone())?;
    unsafe {
        jemalloc_ctl::raw::write(key, value.as_ptr())
            .map_err(|e| anyhow::anyhow!("prof.dump error: {e}"))?;
    }
    Ok(path)
}

fn validate_output_path(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if p.exists() {
        return Err(format!(
            "output path '{path}' already exists; refusing to overwrite"
        ));
    }
    if let Some(parent) = p.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        return Err(format!(
            "parent directory '{}' of output path '{path}' does not exist",
            parent.display()
        ));
    }
    Ok(())
}

pub fn handle_dump_profile_request(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let query = req.uri().query().unwrap_or("");
    let query_pairs: HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes()).collect();
    let output_path = query_pairs
        .get("output")
        .map(|p| p.to_string())
        .filter(|p| !p.is_empty());

    if let Some(path) = output_path.as_deref()
        && let Err(msg) = validate_output_path(path)
    {
        return Ok(reply_with_status(StatusCode::BAD_REQUEST, msg));
    }

    match dump_heap_profile(output_path) {
        Ok(path) => {
            info!("Finished dumping heap profile to {path}.");
            Ok(reply_with(
                Vec::new(),
                Body::from(format!("Successfully dumped heap profile to {path}")),
            ))
        },
        Err(e) => {
            info!("Failed to dump heap profile: {e:?}");
            Ok(reply_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to dump heap profile: {e}"),
            ))
        },
    }
}
