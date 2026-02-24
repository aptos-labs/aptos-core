// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reverse proxy for v1 Poem service.
//!
//! When same-port co-hosting is enabled, the v2 Axum server is the external-facing
//! server for both `/v2/*` and `/v1/*` (and `/`) routes. Requests that don't match
//! a v2 route are forwarded to the internal Poem server via this reverse proxy.
//!
//! Since Poem 3.x and Axum 0.7 both use hyper 1.x / http 1.x, the body types
//! are compatible and we can stream request/response bodies without buffering.

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use std::net::SocketAddr;

/// State for the v1 reverse proxy.
#[derive(Clone)]
pub struct V1Proxy {
    poem_address: SocketAddr,
    client: Client<hyper_util::client::legacy::connect::HttpConnector, Body>,
}

impl V1Proxy {
    pub fn new(poem_address: SocketAddr) -> Self {
        let client = Client::builder(TokioExecutor::new()).build_http();
        Self {
            poem_address,
            client,
        }
    }

    /// Forward a request to the internal Poem v1 server.
    pub async fn forward(&self, req: axum::extract::Request) -> Response {
        let (mut parts, body) = req.into_parts();

        // Rewrite the URI to point to the internal Poem server.
        let path_and_query = parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        let new_uri = format!("http://{}{}", self.poem_address, path_and_query);
        parts.uri = match new_uri.parse() {
            Ok(uri) => uri,
            Err(e) => {
                return (StatusCode::BAD_REQUEST, format!("Invalid proxy URI: {}", e))
                    .into_response();
            },
        };

        let proxy_req = axum::http::Request::from_parts(parts, body);

        match self.client.request(proxy_req).await {
            Ok(resp) => {
                let (parts, incoming) = resp.into_parts();
                // Convert hyper::body::Incoming to axum::body::Body.
                // We collect to bytes to bridge the body types cleanly.
                match incoming.collect().await {
                    Ok(collected) => {
                        let bytes = collected.to_bytes();
                        Response::from_parts(parts, Body::from(bytes))
                    },
                    Err(e) => (
                        StatusCode::BAD_GATEWAY,
                        format!("Failed to read proxy response: {}", e),
                    )
                        .into_response(),
                }
            },
            Err(e) => (
                StatusCode::BAD_GATEWAY,
                format!("Proxy connection error: {}", e),
            )
                .into_response(),
        }
    }
}

/// Axum handler that proxies all requests to the internal Poem v1 server.
pub async fn v1_proxy_fallback(
    axum::extract::State(proxy): axum::extract::State<V1Proxy>,
    req: axum::extract::Request,
) -> Response {
    proxy.forward(req).await
}
