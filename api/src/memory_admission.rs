// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Process-wide memory admission middleware. When process resident memory is at or
//! above the configured cap, new API requests are shed with HTTP 503 before
//! dispatch. The health-check endpoint is exempt; a cap of `0` disables the gate.

use crate::metrics::MEMORY_ADMISSION_REJECTED;
use aptos_api_types::{AptosError, AptosErrorCode};
use poem::{http::StatusCode, Endpoint, IntoResponse, Middleware, Request, Response, Result};
use poem_openapi::payload::Json;

/// Path suffix exempt from the gate so liveness probes always succeed.
const HEALTH_CHECK_PATH_SUFFIX: &str = "/-/healthy";

/// poem `Middleware` wrapping each endpoint with the memory-admission gate.
/// Constructed with the resident-memory cap in bytes (`0` disables the gate).
pub struct MemoryAdmission {
    cap_bytes: usize,
    resident_fn: fn() -> Option<i64>,
}

impl MemoryAdmission {
    pub fn new(cap_bytes: usize) -> Self {
        Self {
            cap_bytes,
            resident_fn: aptos_jemalloc::current_process_resident_bytes,
        }
    }

    #[cfg(test)]
    fn with_resident_fn(cap_bytes: usize, resident_fn: fn() -> Option<i64>) -> Self {
        Self {
            cap_bytes,
            resident_fn,
        }
    }
}

impl<E: Endpoint> Middleware<E> for MemoryAdmission {
    type Output = MemoryAdmissionEndpoint<E>;

    fn transform(&self, inner: E) -> Self::Output {
        MemoryAdmissionEndpoint {
            inner,
            cap_bytes: self.cap_bytes,
            resident_fn: self.resident_fn,
        }
    }
}

pub struct MemoryAdmissionEndpoint<E> {
    inner: E,
    cap_bytes: usize,
    resident_fn: fn() -> Option<i64>,
}

/// Pure admission decision: reject when the gate is enabled (`cap != 0`), the path
/// is not exempt, and a known resident value is at/above the cap. `None` resident
/// (unavailable) fails open.
fn should_reject(resident: Option<i64>, cap: usize, path: &str) -> bool {
    if cap == 0 || path.ends_with(HEALTH_CHECK_PATH_SUFFIX) {
        return false;
    }
    matches!(resident, Some(r) if r >= cap as i64)
}

fn overloaded_response() -> Response {
    Json(AptosError::new_with_error_code(
        "Server is shedding load; please retry shortly",
        AptosErrorCode::Overloaded,
    ))
    .with_status(StatusCode::SERVICE_UNAVAILABLE)
    .into_response()
}

impl<E: Endpoint> Endpoint for MemoryAdmissionEndpoint<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let resident = (self.resident_fn)();
        if should_reject(resident, self.cap_bytes, req.uri().path()) {
            MEMORY_ADMISSION_REJECTED.inc();
            return Ok(overloaded_response());
        }
        self.inner.call(req).await.map(IntoResponse::into_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poem::{handler, http::StatusCode, test::TestClient, EndpointExt};

    #[handler]
    fn ok_handler() -> &'static str {
        "ok"
    }

    fn high_resident() -> Option<i64> {
        Some(1_000_000)
    }

    #[test]
    fn should_reject_logic() {
        // cap 0 -> disabled.
        assert!(!should_reject(Some(100), 0, "/v1/accounts"));
        // unknown resident -> fail open.
        assert!(!should_reject(None, 50, "/v1/accounts"));
        // at/above cap -> reject.
        assert!(should_reject(Some(50), 50, "/v1/accounts"));
        // below cap -> admit.
        assert!(!should_reject(Some(49), 50, "/v1/accounts"));
        // health check exempt even when over cap.
        assert!(!should_reject(Some(1_000), 50, "/v1/-/healthy"));
    }

    #[tokio::test]
    async fn under_cap_passes() {
        // cap (2_000_000) is above the injected resident (1_000_000) -> admit.
        let cli = TestClient::new(
            ok_handler.with(MemoryAdmission::with_resident_fn(2_000_000, high_resident)),
        );
        let resp = cli.get("/v1/accounts").send().await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn cap_zero_disabled_passes() {
        let cli =
            TestClient::new(ok_handler.with(MemoryAdmission::with_resident_fn(0, high_resident)));
        let resp = cli.get("/v1/accounts").send().await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn over_cap_returns_503() {
        let cli = TestClient::new(
            ok_handler.with(MemoryAdmission::with_resident_fn(1000, high_resident)),
        );
        let resp = cli.get("/v1/accounts").send().await;
        resp.assert_status(StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn over_cap_health_check_exempt_passes() {
        let cli = TestClient::new(
            ok_handler.with(MemoryAdmission::with_resident_fn(1000, high_resident)),
        );
        let resp = cli.get("/v1/-/healthy").send().await;
        resp.assert_status_is_ok();
    }
}
