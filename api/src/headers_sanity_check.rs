// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use poem::{
    error::{BadRequest, SizedLimitError},
    http::Method,
    web::headers::{self, HeaderMapExt},
    Endpoint, Middleware, Request, Result,
};

/// This middleware confirms that:
/// 1. If the Content-Length header is set, then it is within the acceptable range.
/// 2. Transfer-Encoding header is not set (as it is not supported).
///
/// Note: this is only applicable to POST requests.
pub struct HeadersSanityCheck {
    max_size: u64,
}

impl HeadersSanityCheck {
    pub fn new(max_size: u64) -> Self {
        Self { max_size }
    }
}

impl<E: Endpoint> Middleware<E> for HeadersSanityCheck {
    type Output = HeadersSanityCheckEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        HeadersSanityCheckEndpoint {
            inner: ep,
            max_size: self.max_size,
        }
    }
}

pub struct HeadersSanityCheckEndpoint<E> {
    inner: E,
    max_size: u64,
}

impl<E: Endpoint> Endpoint for HeadersSanityCheckEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        // If the request method is not POST, skip the checks
        if req.method() != Method::POST {
            return self.inner.call(req).await;
        }

        // If Content-Length is present and exceeds the limit, reject the request
        if let Some(content_length) = req.headers().typed_get::<headers::ContentLength>() {
            if content_length.0 > self.max_size {
                return Err(SizedLimitError::PayloadTooLarge.into());
            }
        }

        // Verify that Transfer-Encoding header is not set (as it is not supported)
        if req
            .headers()
            .typed_get::<headers::TransferEncoding>()
            .is_some()
        {
            return Err(BadRequest(std::io::Error::other(
                "Transfer-Encoding is not supported",
            )));
        }

        self.inner.call(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poem::{handler, http::StatusCode, test::TestClient, EndpointExt};

    // Small size limit (for testing)
    const TEST_SIZE_LIMIT: u64 = 1024; // 1 KB

    #[tokio::test]
    async fn test_post_content_length_within_limit_passes() {
        let cli = TestClient::new(ok_handler.with(HeadersSanityCheck::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .post("/")
            .header("content-length", "50")
            .body(vec![0u8; 50])
            .send()
            .await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_post_content_length_over_limit_rejected() {
        let cli = TestClient::new(ok_handler.with(HeadersSanityCheck::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .post("/")
            .header("content-length", (TEST_SIZE_LIMIT + 1).to_string())
            .body(vec![0u8; TEST_SIZE_LIMIT as usize + 1])
            .send()
            .await;
        resp.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_post_missing_content_length_passes() {
        let cli = TestClient::new(ok_handler.with(HeadersSanityCheck::new(TEST_SIZE_LIMIT)));
        let resp = cli.post("/").body(vec![0u8; 50]).send().await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_post_transfer_encoding_rejected() {
        let cli = TestClient::new(ok_handler.with(HeadersSanityCheck::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .post("/")
            .header("transfer-encoding", "chunked")
            .send()
            .await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_transfer_encoding_not_checked() {
        // HeadersSanityCheck only applies to POST
        let cli = TestClient::new(ok_handler.with(HeadersSanityCheck::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .get("/")
            .header("transfer-encoding", "chunked")
            .send()
            .await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_get_content_length_over_limit_not_checked() {
        let cli = TestClient::new(ok_handler.with(HeadersSanityCheck::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .get("/")
            .header("content-length", (TEST_SIZE_LIMIT + 1).to_string())
            .send()
            .await;
        resp.assert_status_is_ok();
    }

    /// A simple handler that returns "ok" for testing purposes
    #[handler]
    fn ok_handler() -> &'static str {
        "ok"
    }
}
