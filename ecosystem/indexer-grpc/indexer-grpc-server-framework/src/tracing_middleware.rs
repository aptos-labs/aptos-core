// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tower middleware that extracts W3C Trace Context from incoming gRPC requests
//! and creates a tracing span with the remote context as parent.
//!
//! Apply this layer to a tonic `Server::builder()` to automatically propagate
//! distributed traces from upstream services (e.g. API Gateway).

use http::Request;
use opentelemetry::propagation::Extractor;
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower_layer::Layer;
use tower_service::Service;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Tower layer that extracts trace context from incoming gRPC request headers
/// and instruments the request with a properly-parented span.
#[derive(Clone, Debug)]
pub struct OtelGrpcLayer;

impl<S> Layer<S> for OtelGrpcLayer {
    type Service = OtelGrpcService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OtelGrpcService { inner }
    }
}

/// The service produced by [`OtelGrpcLayer`].
#[derive(Clone, Debug)]
pub struct OtelGrpcService<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for OtelGrpcService<S>
where
    S: Service<Request<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Response: Send + 'static,
    S::Error: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = OtelGrpcFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let parent_ctx = extract_context(req.headers());

        let grpc_path = req.uri().path().to_string();
        let grpc_method = grpc_path.rsplit('/').next().unwrap_or(&grpc_path);

        let span = tracing::info_span!(
            "grpc.server",
            otel.kind = "server",
            rpc.system = "grpc",
            rpc.service = grpc_path.split('/').nth(1).unwrap_or("unknown"),
            rpc.method = grpc_method,
        );
        let _ = span.set_parent(parent_ctx);

        // Clone inner service (standard Tower pattern for concurrent requests).
        let mut inner = self.inner.clone();
        std::mem::swap(&mut self.inner, &mut inner);

        let future = inner.call(req);

        OtelGrpcFuture {
            inner: future,
            span,
        }
    }
}

pin_project! {
    /// Future wrapper that instruments the inner future with the extracted trace span.
    pub struct OtelGrpcFuture<F> {
        #[pin]
        inner: F,
        span: tracing::Span,
    }
}

impl<F, T, E> Future for OtelGrpcFuture<F>
where
    F: Future<Output = Result<T, E>>,
{
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let _enter = this.span.enter();
        this.inner.poll(cx)
    }
}

/// Extracts an OpenTelemetry context from HTTP headers using the globally
/// registered text map propagator (W3C TraceContext).
fn extract_context(headers: &http::HeaderMap) -> opentelemetry::Context {
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(headers))
    })
}

/// Adapter that implements [`Extractor`] for [`http::HeaderMap`].
struct HeaderExtractor<'a>(&'a http::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}
