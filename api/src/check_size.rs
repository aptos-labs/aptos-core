// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use hyper::Method;
use poem::{
    error::SizedLimitError,
    web::headers::{self, HeaderMapExt},
    Endpoint, Middleware, Request, Result,
};

/// This middleware confirms that the Content-Length header is set and the
/// value is within the acceptable range. It only applies to POST requests.
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

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for PostSizeLimitEndpoint<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if req.method() != Method::POST {
            return self.inner.call(req).await;
        }

        let content_length = req
            .headers()
            .typed_get::<headers::ContentLength>()
            .ok_or(SizedLimitError::MissingContentLength)?;

        if content_length.0 > self.max_size {
            return Err(SizedLimitError::PayloadTooLarge.into());
        }

        self.inner.call(req).await
    }
}
