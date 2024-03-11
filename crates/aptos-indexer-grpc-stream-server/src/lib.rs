// Copyright © Aptos Foundation

use tonic::transport::{server::Router, NamedService, Server};

const HTTP2_PING_INTERVAL_DURATION: std::time::Duration = std::time::Duration::from_secs(60);
const HTTP2_PING_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

pub struct GrpcServerBuilder<S>
where
    S: NamedService
        + Send
        + Sync
        + Clone
        + tonic::codegen::Service<
            tonic::codegen::http::Request<hyper::body::Body>,
            Response = tonic::codegen::http::Response<tonic::body::BoxBody>,
            Error = std::convert::Infallible,
        > + 'static,
    S::Future: Send,
{
    s: S,
}

impl<S> GrpcServerBuilder<S>
where
    S: NamedService
        + Send
        + Sync
        + Clone
        + tonic::codegen::Service<
            tonic::codegen::http::Request<hyper::body::Body>,
            Response = tonic::codegen::http::Response<tonic::body::BoxBody>,
            Error = std::convert::Infallible,
        > + 'static,
    S::Future: Send,
{
    pub fn new(s: S) -> Self {
        Self { s }
    }

    pub fn build_router(
        &self,
        tls_config: Option<tonic::transport::ServerTlsConfig>,
        file_descriptors: &[&[u8]],
    ) -> Router {
        let svc = self.s.clone();
        let mut reflection_service_builder = tonic_reflection::server::Builder::configure();
        for file_descriptor in file_descriptors {
            // Note: It is critical that the file descriptor set is registered for every
            // file that the top level API proto depends on recursively. If you don't,
            // compilation will still succeed but reflection will fail at runtime.
            //
            // TODO: Add a test for this / something in build.rs, this is a big footgun.
            reflection_service_builder =
                reflection_service_builder.register_encoded_file_descriptor_set(file_descriptor);
        }

        let mut builder = Server::builder()
            .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
            .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION));

        if let Some(tls_config) = tls_config {
            builder = builder.tls_config(tls_config).unwrap();
        };

        let reflection_service = reflection_service_builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build reflection service: {}", e))
            .unwrap();

        builder.add_service(svc).add_service(reflection_service)
    }
}
