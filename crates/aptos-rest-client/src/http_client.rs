// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use reqwest::{
    header::{HeaderMap, HeaderName},
    Client as ReqwestClient, IntoUrl, RequestBuilder as ReqwestRequestBuilder, Response,
};
use serde::Serialize;

#[derive(Clone, Debug)]
pub(crate) struct HttpClient {
    inner: ReqwestClient,
    headers: HeaderMap,
}

/// Wrapper around [`reqwest::RequestBuilder`] that exposes no `.send()` method of its own,
/// guaranteeing at compile time that every request is dispatched through [`HttpClient::send`]
/// where per-client headers are attached.
#[derive(Debug)]
pub(crate) struct PreparedRequest(ReqwestRequestBuilder);

impl PreparedRequest {
    pub(crate) fn header(self, key: HeaderName, value: &'static str) -> Self {
        Self(self.0.header(key, value))
    }

    pub(crate) fn query<T: Serialize + ?Sized>(self, query: &T) -> Self {
        Self(self.0.query(query))
    }

    pub(crate) fn body<T: Into<reqwest::Body>>(self, body: T) -> Self {
        Self(self.0.body(body))
    }

    pub(crate) fn json<T: Serialize + ?Sized>(self, json: &T) -> Self {
        Self(self.0.json(json))
    }
}

pub(crate) fn merge_missing_headers(dst: &mut HeaderMap, extra: &HeaderMap) {
    for name in extra.keys() {
        if !dst.contains_key(name) {
            for value in extra.get_all(name) {
                dst.append(name.clone(), value.clone());
            }
        }
    }
}

impl HttpClient {
    pub(crate) fn new(inner: ReqwestClient) -> Self {
        Self {
            inner,
            headers: HeaderMap::new(),
        }
    }

    pub(crate) fn with_headers(&self, headers: HeaderMap) -> Self {
        Self {
            inner: self.inner.clone(),
            headers,
        }
    }

    pub(crate) fn get<U: IntoUrl>(&self, url: U) -> PreparedRequest {
        PreparedRequest(self.inner.get(url))
    }

    pub(crate) fn post<U: IntoUrl>(&self, url: U) -> PreparedRequest {
        PreparedRequest(self.inner.post(url))
    }

    pub(crate) async fn send(&self, request: PreparedRequest) -> Result<Response, reqwest::Error> {
        if self.headers.is_empty() {
            return request.0.send().await;
        }

        let mut req = request.0.build()?;
        merge_missing_headers(req.headers_mut(), &self.headers);
        self.inner.execute(req).await
    }
}
