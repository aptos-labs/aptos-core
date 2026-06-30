// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_logger::info;
use aptos_system_utils::utils::{reply_with, reply_with_status};
#[cfg(target_os = "linux")]
use hyper::header::{HeaderValue, CONTENT_ENCODING, CONTENT_TYPE};
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

pub async fn handle_dump_profile_request(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let query = req.uri().query().unwrap_or("");
    let query_pairs: HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes()).collect();

    // ?format=path keeps the legacy behavior: write the raw .heap file to disk and
    // return its path as text. Default returns a symbolized, gzipped pprof body.
    let want_path_only = query_pairs
        .get("format")
        .map(|f| f.as_ref() == "path")
        .unwrap_or(false);

    let output_path = query_pairs
        .get("output")
        .map(|p| p.to_string())
        .filter(|p| !p.is_empty());

    if let Some(path) = output_path.as_deref()
        && let Err(msg) = validate_output_path(path)
    {
        return Ok(reply_with_status(StatusCode::BAD_REQUEST, msg));
    }

    if want_path_only || output_path.is_some() {
        return Ok(legacy_dump_to_path(output_path));
    }

    #[cfg(target_os = "linux")]
    {
        match dump_pprof_symbolized().await {
            Ok(pprof) => {
                info!("Returning symbolized pprof ({} bytes).", pprof.len());
                let mut resp = Response::new(Body::from(pprof));
                resp.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static("application/octet-stream"),
                );
                resp.headers_mut()
                    .insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
                Ok(resp)
            },
            Err(e) => {
                info!("Failed to produce pprof: {e:?}");
                Ok(reply_with_status(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to produce pprof: {e}"),
                ))
            },
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(legacy_dump_to_path(output_path))
    }
}

fn legacy_dump_to_path(output_path: Option<String>) -> Response<Body> {
    match dump_heap_profile(output_path) {
        Ok(path) => {
            info!("Finished dumping heap profile to {path}.");
            reply_with(
                Vec::new(),
                Body::from(format!("Successfully dumped heap profile to {path}")),
            )
        },
        Err(e) => {
            info!("Failed to dump heap profile: {e:?}");
            reply_with_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to dump heap profile: {e}"),
            )
        },
    }
}

#[cfg(target_os = "linux")]
async fn dump_pprof_symbolized() -> anyhow::Result<Vec<u8>> {
    let prof_ctl_lock = jemalloc_pprof::PROF_CTL
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("jemalloc profiling controller unavailable"))?;
    // Take an owned guard so it can move into the blocking task below.
    let mut prof_ctl = prof_ctl_lock.clone().lock_owned().await;
    if !prof_ctl.activated() {
        anyhow::bail!(
            "jemalloc profiling is not activated; start aptos-node with MALLOC_CONF=prof:true"
        );
    }
    // Symbolization reads /proc/self/maps and parses ELF; it can take seconds.
    // Offload to the blocking pool so other admin-service requests aren't stalled.
    tokio::task::spawn_blocking(move || prof_ctl.dump_pprof())
        .await
        .map_err(|e| anyhow::anyhow!("dump_pprof task join error: {e}"))?
}
