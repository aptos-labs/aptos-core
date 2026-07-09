// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use poem::{
    error::{ReadBodyError, SizedLimitError},
    http::Method,
    web::{headers, headers::HeaderMapExt},
    Body, Endpoint, Middleware, Request, Result,
};

/// This middleware confirms that:
/// 1. If the Content-Length header is set, then it is within the acceptable range.
/// 2. The actual size of the request body is within the acceptable range.
///
/// Note: this is only applicable to POST requests.
pub struct PostSizeLimit {
    max_size: u64,
}

impl PostSizeLimit {
    pub fn new(max_size: u64) -> Self {
        Self { max_size }
    }
}

impl<E: Endpoint> Middleware<E> for PostSizeLimit {
    type Output = PostSizeLimitEndpoint<E>;

    fn transform(&self, ep: E) -> Self::Output {
        PostSizeLimitEndpoint {
            inner: ep,
            max_size: self.max_size,
        }
    }
}

/// Endpoint for PostSizeLimit middleware.
pub struct PostSizeLimitEndpoint<E> {
    inner: E,
    max_size: u64,
}

impl<E: Endpoint> Endpoint for PostSizeLimitEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
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

        // Verify the request body size is within the limit
        match req
            .take_body()
            .into_bytes_limit(self.max_size as usize)
            .await
        {
            Ok(bytes) => {
                req.set_body(Body::from_bytes(bytes));
                self.inner.call(req).await
            },
            Err(ReadBodyError::PayloadTooLarge) => Err(SizedLimitError::PayloadTooLarge.into()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headers_sanity_check::HeadersSanityCheck;
    use flate2::{write::GzEncoder, Compression as GzCompression};
    use poem::{handler, http::StatusCode, middleware::Compression, test::TestClient, EndpointExt};
    use std::io::Write;

    // Small size limit (for testing)
    const TEST_SIZE_LIMIT: u64 = 1024; // 1 KB

    #[tokio::test]
    async fn test_post_within_limit_passes() {
        let cli = TestClient::new(ok_handler.with(PostSizeLimit::new(TEST_SIZE_LIMIT)));
        let resp = cli.post("/").body(vec![0u8; 50]).send().await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_post_at_exact_limit_passes() {
        let cli = TestClient::new(ok_handler.with(PostSizeLimit::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .post("/")
            .body(vec![0u8; TEST_SIZE_LIMIT as usize])
            .send()
            .await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_post_over_limit_rejected() {
        let cli = TestClient::new(ok_handler.with(PostSizeLimit::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .post("/")
            .body(vec![0u8; TEST_SIZE_LIMIT as usize + 1])
            .send()
            .await;
        resp.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_post_content_length_within_limit_passes() {
        let cli = TestClient::new(ok_handler.with(PostSizeLimit::new(TEST_SIZE_LIMIT)));
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
        let cli = TestClient::new(ok_handler.with(PostSizeLimit::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .post("/")
            .header("content-length", (TEST_SIZE_LIMIT + 1).to_string())
            .body(vec![0u8; TEST_SIZE_LIMIT as usize + 1])
            .send()
            .await;
        resp.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_get_over_limit_not_checked() {
        let cli = TestClient::new(ok_handler.with(PostSizeLimit::new(TEST_SIZE_LIMIT)));
        let resp = cli
            .get("/")
            .body(vec![0u8; TEST_SIZE_LIMIT as usize * 10])
            .send()
            .await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_uncompressed_post_over_limit_rejected() {
        let cli = TestClient::new(full_stack());
        let resp = cli
            .post("/")
            .body(vec![0u8; TEST_SIZE_LIMIT as usize + 1])
            .send()
            .await;
        resp.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_gzip_body_within_decompressed_limit_passes() {
        let cli = TestClient::new(full_stack());
        let compressed = gzip(&vec![b'A'; (TEST_SIZE_LIMIT / 2) as usize]);
        let resp = cli
            .post("/")
            .header("content-encoding", "gzip")
            .body(compressed)
            .send()
            .await;
        resp.assert_status_is_ok();
    }

    #[tokio::test]
    async fn test_gzip_decompressed_over_limit_rejected() {
        let cli = TestClient::new(full_stack());
        let compressed = gzip(&vec![b'A'; TEST_SIZE_LIMIT as usize * 100]);

        // Verify that the compressed body is actually smaller than the limit
        assert!(compressed.len() < TEST_SIZE_LIMIT as usize,);

        // Now send the request with the compressed body and expect it to be rejected due to decompressed size
        let resp = cli
            .post("/")
            .header("content-encoding", "gzip")
            .body(compressed)
            .send()
            .await;
        resp.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_content_length_over_limit_fast_rejected_by_headers_check() {
        let cli = TestClient::new(full_stack());
        let resp = cli
            .post("/")
            .header("content-length", (TEST_SIZE_LIMIT + 1).to_string())
            .body(vec![0u8; TEST_SIZE_LIMIT as usize + 1])
            .send()
            .await;
        resp.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
    }

    /// A simple handler that returns "ok" for testing purposes
    #[handler]
    fn ok_handler() -> &'static str {
        "ok"
    }

    /// Helper function to create a full middleware stack for testing.
    /// Note: Poem uses LIFO chaining: .with(A).with(B).with(C) executes as C → B → A → handler.
    fn full_stack() -> impl Endpoint<Output = poem::Response> {
        ok_handler
            .with(PostSizeLimit::new(TEST_SIZE_LIMIT))
            .with(Compression::new())
            .with(HeadersSanityCheck::new(TEST_SIZE_LIMIT))
    }

    /// Helper function to gzip-compress data for testing
    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut enc = GzEncoder::new(Vec::new(), GzCompression::best());
        enc.write_all(data).unwrap();
        enc.finish().unwrap()
    }
}
