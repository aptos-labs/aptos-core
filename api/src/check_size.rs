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
    inner: E,
    max_size: u64,
}

impl PostSizeLimit {
    pub fn new(inner: E, max_size: u64) -> Self {
        Self { inner, max_size }
    }
}

impl<E: Endpoint> Middleware<E> for PostSizeLimit {
    type Output = Self;

    fn transform(&self, ep: E) -> Self::Output {
        Self { inner: ep, max_size: self.max_size }
    }
}

#[async_trait::async_trait]
impl<E: Endpoint> Endpoint for PostSizeLimit {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        if !req.method().is_post() {
            return self.inner.call(req).await;
        }

        let content_length = match req.headers().typed_try_get::<headers::ContentLength>() {
            Some(content_length) => content_length,
            None => return Err(SizedLimitError::MissingContentLength.into()),
        };

        if content_length.0 > self.max_size {
            return Err(SizedLimitError::PayloadTooLarge.into());
        }

        self.inner.call(req).await
    }
}
