// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::config::IndexerGrpcGatewayConfig;
use anyhow::Context;
use aptos_indexer_grpc_utils::trace_context::{
    parse_traceparent, set_otel_parent, trace_context_from_current_otel_span, TraceContext,
    TRACEPARENT_HEADER, TRACESTATE_HEADER,
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
use tracing::{debug, info, info_span, warn, Instrument};
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

fn extract_trace_context_from_headers(headers: &axum::http::HeaderMap) -> TraceContext {
    let ctx = headers
        .get(TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_traceparent)
        .map(|mut ctx| {
            ctx.tracestate = headers
                .get(TRACESTATE_HEADER)
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            ctx
        });
    ctx.unwrap_or_else(TraceContext::new_root)
}

fn inject_trace_context_into_headers(headers: &mut axum::http::HeaderMap, ctx: &TraceContext) {
    if let Ok(val) = ctx.to_traceparent().parse() {
        headers.insert(TRACEPARENT_HEADER, val);
    }
    if let Some(ref tracestate) = ctx.tracestate {
        if let Ok(val) = tracestate.parse() {
            headers.insert(TRACESTATE_HEADER, val);
        }
    }
}

async fn get_data_service_url(
    State(config): State<Arc<IndexerGrpcGatewayConfig>>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let trace_ctx = extract_trace_context_from_headers(req.headers());
    let span = info_span!(
        "grpc_gateway.get_data_service_url",
        trace_id = %trace_ctx.trace_id,
        parent_span_id = %trace_ctx.parent_span_id,
        path = %req.uri().path(),
        otel.kind = "server",
    );
    set_otel_parent(&span, &trace_ctx);

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
        let parse_span = info_span!("grpc_gateway.parse_request_body");
        let (parsed_request, new_body) = async {
            let body_bytes = body
                .collect()
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .to_bytes();
            let restored_body = body_bytes.clone().into();
            let stream =
                Streaming::<GetTransactionsRequest>::new_request(
                    <ProstCodec<GetTransactionsRequest, GetTransactionsRequest> as Codec>::decoder(
                        &mut tonic::codec::ProstCodec::<
                            GetTransactionsRequest,
                            GetTransactionsRequest,
                        >::default(),
                    ),
                    Full::new(body_bytes),
                    request_compression_encoding,
                    None,
                );

            tokio::pin!(stream);

            let parsed = match stream.try_next().await {
                Ok(Some(request)) => Some(request),
                _ => None,
            };
            Ok::<_, (StatusCode, String)>((parsed, restored_body))
        }
        .instrument(parse_span)
        .await?;
        user_request = parsed_request;
        body = new_body;
    }

    let child_ctx = trace_ctx.new_child();
    let response =
        call_grpc_manager(&config.grpc_manager_address, user_request, &child_ctx).await?;

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

        let outgoing_ctx =
            trace_context_from_current_otel_span().unwrap_or_else(|| trace_ctx.new_child());
        let mut grpc_manager_request =
            tonic::Request::new(GetDataServiceForRequestRequest { user_request });
        aptos_indexer_grpc_utils::trace_context::inject_trace_context_into_request(
            &mut grpc_manager_request,
            &outgoing_ctx,
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
        debug!(
            data_service_url = data_service_url.as_str(),
            "Proxying request to data service"
        );
        *request.uri_mut() = override_uri_with_upstream_url(request.uri(), &data_service_url)?;

        let outgoing_ctx =
            trace_context_from_current_otel_span().unwrap_or_else(|| trace_ctx.new_child());
        inject_trace_context_into_headers(request.headers_mut(), &outgoing_ctx);

        let result = Client::builder(TokioExecutor::new())
            .http2_only(true)
            .build_http()
            .request(request)
            .await;

        match result {
            Ok(res) => {
                let status = res.status();
                debug!(
                    upstream_status = %status,
                    "Data service proxy response"
                );
                let (parts, body) = res.into_parts();
                Ok(Response::from_parts(parts, axum::body::Body::new(body)))
            },
            Err(e) => {
                warn!(error = %e, "Data service proxy error");
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            },
        }
    }
    .instrument(span)
    .await
}
