// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::IndexerGrpcGatewayConfig;
use anyhow::Context;
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
use tracing::info;
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

async fn get_data_service_url(
    State(config): State<Arc<IndexerGrpcGatewayConfig>>,
    req: Request,
    next: Next,
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

    let mut client = GrpcManagerClient::connect(config.grpc_manager_address.to_string())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let grpc_manager_request =
        tonic::Request::new(GetDataServiceForRequestRequest { user_request });
    let response: GetDataServiceForRequestResponse = client
        .get_data_service_for_request(grpc_manager_request)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .into_inner();

    let url = Url::from_str(&response.data_service_address).unwrap();
    let mut req = Request::from_parts(head, body);
    req.extensions_mut().insert(url);
    Ok(next.run(req).await)
}

async fn proxy(
    data_service_url: Extension<Url>,
    mut request: Request,
) -> Result<Response, (StatusCode, String)> {
    info!(
        data_service_url = data_service_url.as_str(),
        "Proxying request to data service: {}",
        data_service_url.as_str()
    );
    *request.uri_mut() = override_uri_with_upstream_url(request.uri(), &data_service_url)?;

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
