use tonic::{
    service::{interceptor::InterceptedService, Interceptor},
    transport::{server::Router, NamedService, Server},
};

const HTTP2_PING_INTERVAL_DURATION: std::time::Duration = std::time::Duration::from_secs(60);
const HTTP2_PING_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

pub struct GrpcServerBuilder<S, I>
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
    I: Interceptor + Send + Sync + Clone + 'static,
{
    s: S,
    i: I,
}

impl<S, I> GrpcServerBuilder<S, I>
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
    I: Interceptor + Send + Sync + Clone + 'static,
{
    pub fn new(s: S, i: I) -> Self {
        Self { s, i }
    }

    pub fn build_router(
        &self,
        tls_config: Option<tonic::transport::ServerTlsConfig>,
        file_descriptors: &[&[u8]],
    ) -> Router {
        let svc_with_interceptor = InterceptedService::new(self.s.clone(), self.i.clone());
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

        let reflection_service = reflection_service_builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build reflection service: {}", e))
            .unwrap();

        let mut builder = Server::builder()
            .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
            .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION));
        if let Some(tls_config) = tls_config {
            builder = builder.tls_config(tls_config).unwrap();
        };
        builder
            .add_service(svc_with_interceptor)
            .add_service(reflection_service)
    }
}
