// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::config::IndexerGrpcGatewayConfig;
use anyhow::Context;
use aptos_indexer_grpc_utils::trace_context::{
    parse_traceparent, TraceContext, TRACEPARENT_HEADER, TRACESTATE_HEADER,
};
use aptos_protos::indexer::v1::{
    grpc_manager_client::GrpcManagerClient, GetDataServiceForRequestRequest,
    GetDataServiceForRequestResponse, GetTransactionsRequest,
};
use axum::{
    extract::{Request, State},
    http::{StatusCode, Uri},
    middleware::{from_fn_with_state, Next},
    response::Response,
    routing::any,
    Extension, Router,
};
use futures::TryStreamExt;
use http_body_util::{BodyExt, Full};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use std::{str::FromStr, sync::Arc};
use tonic::{
    codec::{Codec, CompressionEncoding, ProstCodec},
    Streaming,
};
use tracing::{info, info_span, Instrument};
use url::Url;

const LISTEN_ADDRESS: &str = "0.0.0.0";
const ENCODING_HEADER: &str = "grpc-encoding";

pub(crate) struct GrpcGateway {
    config: Arc<IndexerGrpcGatewayConfig>,
}

impl GrpcGateway {
    pub(crate) fn new(config: IndexerGrpcGatewayConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub(crate) async fn start(&self) -> anyhow::Result<()> {
        let app = Router::new()
            .route("/*path", any(proxy).with_state(self.config.clone()))
            .layer(from_fn_with_state(
                self.config.clone(),
                get_data_service_url,
            ));

        info!(
            "gRPC Gateway listening on {}:{}",
            LISTEN_ADDRESS, self.config.port
        );
        let listener = tokio::net::TcpListener::bind((LISTEN_ADDRESS, self.config.port))
            .await
            .expect("Failed to bind TCP listener");

        axum::serve(listener, app)
            .await
            .context("Failed to serve gRPC Gateway")
    }
}

fn override_uri_with_upstream_url(
    original_uri: &Uri,
    upstream_url: &Url,
) -> Result<Uri, (StatusCode, String)> {
    let requested_path_and_query = original_uri.path_and_query().unwrap().to_string();

    let new_url = upstream_url
        .join(requested_path_and_query.as_str())
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                format!("Requested URL is not supported: {}", original_uri),
            )
        })?;

    if new_url.origin() != upstream_url.origin() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Requested URL is not supported: {}", original_uri),
        ));
    }

    let uri = Uri::try_from(new_url.as_str()).unwrap();

    Ok(uri)
}

/// Extracts the incoming W3C Trace Context from request headers.
fn extract_trace_context_from_request(req: &Request) -> TraceContext {
    let traceparent = req
        .headers()
        .get(TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_traceparent);

    match traceparent {
        Some(mut ctx) => {
            ctx.tracestate = req
                .headers()
                .get(TRACESTATE_HEADER)
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            ctx
        },
        None => TraceContext::new_root(),
    }
}

async fn get_data_service_url(
    State(config): State<Arc<IndexerGrpcGatewayConfig>>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let trace_ctx = extract_trace_context_from_request(&req);
    let span = info_span!(
        "grpc_gateway.get_data_service_url",
        trace_id = %trace_ctx.trace_id,
        parent_span_id = %trace_ctx.parent_span_id,
        path = %req.uri().path(),
        otel.kind = "server",
    );

    get_data_service_url_inner(config, req, next, trace_ctx)
        .instrument(span)
        .await
}

async fn get_data_service_url_inner(
    config: Arc<IndexerGrpcGatewayConfig>,
    req: Request,
    next: Next,
    trace_ctx: TraceContext,
) -> Result<Response, (StatusCode, String)> {
    let request_compression_encoding: Option<CompressionEncoding> = req
        .headers()
        .get(ENCODING_HEADER)
        .and_then(|encoding_header| {
            encoding_header
                .to_str()
                .ok()
                .map(|encoding_str| match encoding_str {
                    "gzip" => Some(CompressionEncoding::Gzip),
                    "zstd" => Some(CompressionEncoding::Zstd),
                    _ => None,
                })
        })
        .flatten();

    let (head, mut body) = req.into_parts();

    let mut user_request = None;
    if head.uri.path() == "/aptos.indexer.v1.RawData/GetTransactions" {
        let body_bytes = body
            .collect()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .to_bytes();
        body = body_bytes.clone().into();
        let stream = Streaming::<GetTransactionsRequest>::new_request(
            <ProstCodec<GetTransactionsRequest, GetTransactionsRequest> as Codec>::decoder(
                &mut tonic::codec::ProstCodec::<GetTransactionsRequest, GetTransactionsRequest>::default(),
            ),
            Full::new(body_bytes),
            request_compression_encoding,
            None,
        );

        tokio::pin!(stream);

        if let Ok(Some(request)) = stream.try_next().await {
            user_request = Some(request);
        }
    }

    let child_ctx = trace_ctx.new_child();
    let response = call_grpc_manager(
        &config.grpc_manager_address,
        user_request,
        &child_ctx,
    )
    .await?;

    let url = Url::from_str(&response.data_service_address).unwrap();
    let mut req = Request::from_parts(head, body);
    req.extensions_mut().insert(url);
    req.extensions_mut().insert(trace_ctx);
    Ok(next.run(req).await)
}

/// Calls the gRPC manager to resolve the data service address, propagating trace context.
async fn call_grpc_manager(
    grpc_manager_address: &str,
    user_request: Option<GetTransactionsRequest>,
    trace_ctx: &TraceContext,
) -> Result<GetDataServiceForRequestResponse, (StatusCode, String)> {
    let span = info_span!(
        "grpc_gateway.call_grpc_manager",
        trace_id = %trace_ctx.trace_id,
        parent_span_id = %trace_ctx.parent_span_id,
        otel.kind = "client",
        grpc_manager_address = grpc_manager_address,
    );

    async {
        let mut client = GrpcManagerClient::connect(grpc_manager_address.to_string())
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let mut grpc_manager_request =
            tonic::Request::new(GetDataServiceForRequestRequest { user_request });
        aptos_indexer_grpc_utils::trace_context::inject_trace_context_into_request(
            &mut grpc_manager_request,
            trace_ctx,
        );

        let response: GetDataServiceForRequestResponse = client
            .get_data_service_for_request(grpc_manager_request)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .into_inner();

        info!(
            data_service_address = %response.data_service_address,
            "Resolved data service address from gRPC manager"
        );

        Ok(response)
    }
    .instrument(span)
    .await
}

async fn proxy(
    data_service_url: Extension<Url>,
    trace_ctx: Option<Extension<TraceContext>>,
    mut request: Request,
) -> Result<Response, (StatusCode, String)> {
    let trace_ctx = trace_ctx
        .map(|ext| ext.0)
        .unwrap_or_else(TraceContext::new_root);

    let span = info_span!(
        "grpc_gateway.proxy",
        trace_id = %trace_ctx.trace_id,
        parent_span_id = %trace_ctx.parent_span_id,
        otel.kind = "client",
        data_service_url = data_service_url.as_str(),
    );

    async {
        info!(
            data_service_url = data_service_url.as_str(),
            "Proxying request to data service"
        );
        *request.uri_mut() = override_uri_with_upstream_url(request.uri(), &data_service_url)?;

        // Ensure trace context is forwarded on the proxied request.
        let child_ctx = trace_ctx.new_child();
        if let Ok(val) = child_ctx.to_traceparent().parse() {
            request.headers_mut().insert(TRACEPARENT_HEADER, val);
        }
        if let Some(ref tracestate) = child_ctx.tracestate {
            if let Ok(val) = tracestate.parse() {
                request.headers_mut().insert(TRACESTATE_HEADER, val);
            }
        }

        Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http()
            .request(request)
            .await
            .map(|res| {
                let (parts, body) = res.into_parts();
                Response::from_parts(parts, axum::body::Body::new(body))
            })
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    }
    .instrument(span)
    .await
}
