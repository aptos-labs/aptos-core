// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Linked MonoVM execution adapter for the Lean differential test harness.
//!
//! This crate is the Rust half of the Lean/MonoVM boundary described in
//! `third_party/move/lean/designs/monovm-link-design.md`. It exports a
//! minimal versioned C ABI that a small C shim, written against the pinned
//! Lean toolchain's `lean/lean.h`, links into the Lean test executable:
//!
//! ```c
//! uint32_t leaner_monovm_abi_version(void);
//! int32_t leaner_monovm_run(
//!     const uint8_t *request, size_t request_len,
//!     struct leaner_buffer *response);
//! void leaner_monovm_buffer_free(struct leaner_buffer response);
//! ```
//!
//! Only owned bytes cross the boundary. Every domain failure — compilation,
//! loading, execution, a malformed request — travels *inside* the response
//! payload; the `int32_t` status is transport-only. Panics are caught and
//! encoded as internal harness errors, which relies on unwinding, so the
//! crate refuses to compile under a `panic = "abort"` profile.

#[cfg(panic = "abort")]
compile_error!(
    "mono-move-lean-link contains MonoMove panics with catch_unwind before they \
     reach the C boundary; a panic=abort profile would let them cross into the \
     Lean runtime"
);

pub mod engine;
pub mod marshal;
pub mod payload;

use payload::{Outcome, Request, Response, Stage, PAYLOAD_VERSION};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Native ABI version of the C boundary. Bump when an exported function's
/// signature, the `LeanerBuffer` layout, or the containment contract
/// changes; the payload schema has its own independent version.
pub const ABI_VERSION: u32 = 1;

/// An owning byte buffer crossing the C boundary. Produced by
/// [`leaner_monovm_run`] and returned to [`leaner_monovm_buffer_free`]; the
/// bytes are always a `payload::Response`.
#[repr(C)]
pub struct LeanerBuffer {
    pub data: *mut u8,
    pub len: usize,
}

impl LeanerBuffer {
    fn from_vec(bytes: Vec<u8>) -> Self {
        let bytes = bytes.into_boxed_slice();
        let len = bytes.len();
        let data = Box::into_raw(bytes) as *mut u8;
        Self { data, len }
    }

    fn null() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }

    fn into_vec(self) -> Vec<u8> {
        if self.data.is_null() {
            return Vec::new();
        }
        // SAFETY: the only producer of a non-null buffer is `from_vec`,
        // which boxed a length-exact slice, so this reconstructs it.
        let slice = unsafe { std::slice::from_raw_parts_mut(self.data, self.len) };
        let boxed = unsafe { Box::from_raw(slice as *mut [u8]) };
        boxed.into_vec()
    }
}

/// Reports the native ABI version of this library.
#[unsafe(no_mangle)]
pub extern "C" fn leaner_monovm_abi_version() -> u32 {
    ABI_VERSION
}

/// Runs one request. Returns `0` when `*response` holds the encoded
/// response and the caller owes it to `leaner_monovm_buffer_free`; a
/// nonzero status means no response could be produced at all and
/// `*response` is a null buffer.
///
/// # Safety
///
/// `request` must be readable for `request_len` bytes when it is non-null,
/// and `response` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn leaner_monovm_run(
    request: *const u8,
    request_len: usize,
    response: *mut LeanerBuffer,
) -> i32 {
    if response.is_null() {
        return 1;
    }
    unsafe { *response = LeanerBuffer::null() };
    let request_bytes = if request.is_null() {
        &[][..]
    } else {
        // SAFETY: the caller guarantees `request_len` readable bytes.
        unsafe { std::slice::from_raw_parts(request, request_len) }
    };
    let bytes = contain(
        || run_bytes(request_bytes),
        |message| encode_response(&internal_response(message)),
    );
    unsafe { *response = LeanerBuffer::from_vec(bytes) };
    0
}

/// Frees a response buffer produced by [`leaner_monovm_run`]. Passing a
/// null buffer is a no-op.
///
/// # Safety
///
/// `response` must be a buffer produced by this library and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn leaner_monovm_buffer_free(response: LeanerBuffer) {
    let _ = response.into_vec();
}

/// Runs one encoded request to its encoded response: the exact path the C
/// ABI wraps, and the entry the Rust tests drive.
pub fn run_bytes(request: &[u8]) -> Vec<u8> {
    let response = match serde_json::from_slice::<Request>(request) {
        Ok(request) if request.version == PAYLOAD_VERSION => {
            engine::run_request(&request, ABI_VERSION)
        },
        Ok(request) => Response::uniform_error(
            &request,
            &engine::identity(ABI_VERSION),
            Stage::Abi,
            format!(
                "payload version {} is not supported; expected {PAYLOAD_VERSION}",
                request.version
            ),
        ),
        Err(error) => malformed_request_response(&error),
    };
    encode_response(&response)
}

/// Runs `inner` with panic containment: a caught panic becomes the response
/// produced by `on_panic` instead of unwinding through the caller.
fn contain(inner: impl FnOnce() -> Vec<u8>, on_panic: impl FnOnce(String) -> Vec<u8>) -> Vec<u8> {
    match catch_unwind(AssertUnwindSafe(inner)) {
        Ok(bytes) => bytes,
        Err(panic) => on_panic(panic_message(&panic)),
    }
}

/// A response for a request that could not be decoded at all. The call
/// count is unknown, so the response carries the single failure.
fn malformed_request_response(error: &serde_json::Error) -> Response {
    Response {
        version: PAYLOAD_VERSION,
        identity: engine::identity(ABI_VERSION),
        outcomes: vec![Outcome::Error {
            stage: Stage::Abi,
            message: format!("malformed request payload: {error}"),
        }],
    }
}

/// A response reporting a contained panic as an internal harness error.
fn internal_response(message: String) -> Response {
    Response {
        version: PAYLOAD_VERSION,
        identity: engine::identity(ABI_VERSION),
        outcomes: vec![Outcome::Error {
            stage: Stage::Internal,
            message,
        }],
    }
}

/// Extracts a panic message when the payload carries one.
///
/// A `panic!` payload is a `&str` or `String`; a panic that was resumed
/// before `catch_unwind` keeps its original payload boxed once more, so that
/// layer is tried too.
fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    payload_text(panic).map_or_else(
        || "mono-move panicked".to_string(),
        |message| format!("mono-move panicked: {message}"),
    )
}

/// The bare text carried by a panic payload, if any.
fn payload_text(payload: &(dyn std::any::Any + Send)) -> Option<String> {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return Some((*message).to_string());
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return Some(message.clone());
    }
    payload
        .downcast_ref::<Box<dyn std::any::Any + Send>>()
        .and_then(|inner| payload_text(inner.as_ref()))
}

/// Encodes a response deterministically. Serialization of the payload types
/// is infallible in memory, so the expect is unreachable.
fn encode_response(response: &Response) -> Vec<u8> {
    serde_json::to_vec(response).expect("response serialization is infallible")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_caught_panic_becomes_an_internal_error_response() {
        let bytes = contain(
            || panic!("boom"),
            |message| encode_response(&internal_response(message)),
        );
        let response: Response = serde_json::from_slice(&bytes).expect("valid response");
        assert_eq!(response.version, PAYLOAD_VERSION);
        assert_eq!(response.outcomes, vec![Outcome::Error {
            stage: Stage::Internal,
            message: "mono-move panicked: boom".to_string(),
        }]);
    }

    #[test]
    fn a_malformed_request_becomes_an_abi_error_response() {
        let bytes = run_bytes(b"this is not json");
        let response: Response = serde_json::from_slice(&bytes).expect("valid response");
        assert!(matches!(response.outcomes.as_slice(), [Outcome::Error {
            stage: Stage::Abi,
            ..
        }]));
    }

    #[test]
    fn an_unsupported_payload_version_becomes_an_abi_error_response() {
        let request = r#"{
            "version": 99,
            "compile": {"sources": [], "language": 2},
            "limits": {"gas": 1000},
            "calls": [{"function": "0x1::m::f", "args": []}]
        }"#;
        let bytes = run_bytes(request.as_bytes());
        let response: Response = serde_json::from_slice(&bytes).expect("valid response");
        assert!(matches!(response.outcomes.as_slice(), [Outcome::Error {
            stage: Stage::Abi,
            ..
        }]));
    }
}
