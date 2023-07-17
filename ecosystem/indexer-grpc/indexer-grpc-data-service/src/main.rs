// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_data_service::service::RawDataServerWrapper;
use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use aptos_indexer_grpc_utils::config::IndexerGrpcFileStoreConfig;
use aptos_protos::{
    indexer::v1::FILE_DESCRIPTOR_SET as INDEXER_V1_FILE_DESCRIPTOR_SET,
    transaction::v1::FILE_DESCRIPTOR_SET as TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET,
    util::timestamp::FILE_DESCRIPTOR_SET as UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, net::ToSocketAddrs};
use tonic::{
    codec::CompressionEncoding,
    codegen::InterceptedService,
    metadata::{Ascii, MetadataValue},
    transport::Server,
    Request, Status,
};

// HTTP2 ping interval and timeout.
// This can help server to garbage collect dead connections.
// tonic server: https://docs.rs/tonic/latest/tonic/transport/server/struct.Server.html#method.http2_keepalive_interval
const HTTP2_PING_INTERVAL_DURATION: std::time::Duration = std::time::Duration::from_secs(60);
const HTTP2_PING_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    // TLS config.
    pub data_service_grpc_listen_address: String,
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonTlsConfig {
    pub data_service_grpc_listen_address: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcDataServiceConfig {
    // The address for TLS and non-TLS gRPC server to listen on.
    pub data_service_grpc_tls_config: Option<TlsConfig>,
    pub data_service_grpc_non_tls_config: Option<NonTlsConfig>,
    // A list of auth tokens that are allowed to access the service.
    pub whitelisted_auth_tokens: Vec<String>,
    // File store config.
    pub file_store_config: IndexerGrpcFileStoreConfig,
    // Redis read replica address.
    pub redis_read_replica_address: String,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcDataServiceConfig {
    async fn run(&self) -> Result<()> {
        let token_set = build_auth_token_set(self.whitelisted_auth_tokens.clone());
        let authentication_inceptor =
            move |req: Request<()>| -> std::result::Result<Request<()>, Status> {
                let metadata = req.metadata();
                if let Some(token) =
                    metadata.get(aptos_indexer_grpc_utils::constants::GRPC_AUTH_TOKEN_HEADER)
                {
                    if token_set.contains(token) {
                        std::result::Result::Ok(req)
                    } else {
                        Err(Status::unauthenticated("Invalid token"))
                    }
                } else {
                    Err(Status::unauthenticated("Missing token"))
                }
            };
        let reflection_service = tonic_reflection::server::Builder::configure()
            // Note: It is critical that the file descriptor set is registered for every
            // file that the top level API proto depends on recursively. If you don't,
            // compilation will still succeed but reflection will fail at runtime.
            //
            // TODO: Add a test for this / something in build.rs, this is a big footgun.
            .register_encoded_file_descriptor_set(INDEXER_V1_FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build reflection service: {}", e))?;

        // Add authentication interceptor.
        let server = RawDataServerWrapper::new(
            self.redis_read_replica_address.clone(),
            self.file_store_config.clone(),
        );
        let svc = aptos_protos::indexer::v1::raw_data_server::RawDataServer::new(server)
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip);
        let svc_with_interceptor = InterceptedService::new(svc, authentication_inceptor);

        let svc_with_interceptor_clone = svc_with_interceptor.clone();
        let reflection_service_clone = reflection_service.clone();

        let mut tasks = vec![];
        if self.data_service_grpc_non_tls_config.is_some() {
            let config = self.data_service_grpc_non_tls_config.clone().unwrap();
            let grpc_address = config
                .data_service_grpc_listen_address
                .to_socket_addrs()
                .map_err(|e| anyhow::anyhow!(e))?
                .next()
                .ok_or_else(|| anyhow::anyhow!("Failed to parse grpc address"))?;
            tasks.push(tokio::spawn(async move {
                Server::builder()
                    .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
                    .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION))
                    .add_service(svc_with_interceptor_clone)
                    .add_service(reflection_service_clone)
                    .serve(grpc_address)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }));
        }
        if self.data_service_grpc_tls_config.is_some() {
            let config = self.data_service_grpc_tls_config.clone().unwrap();
            let grpc_address = config
                .data_service_grpc_listen_address
                .to_socket_addrs()
                .map_err(|e| anyhow::anyhow!(e))?
                .next()
                .ok_or_else(|| anyhow::anyhow!("Failed to parse grpc address"))?;

            let cert = tokio::fs::read(config.cert_path.clone()).await?;
            let key = tokio::fs::read(config.key_path.clone()).await?;
            let identity = tonic::transport::Identity::from_pem(cert, key);
            tasks.push(tokio::spawn(async move {
                Server::builder()
                    .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
                    .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION))
                    .tls_config(tonic::transport::ServerTlsConfig::new().identity(identity))?
                    .add_service(svc_with_interceptor)
                    .add_service(reflection_service)
                    .serve(grpc_address)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }));
        }

        if tasks.is_empty() {
            return Err(anyhow::anyhow!("No grpc config provided"));
        }

        futures::future::try_join_all(tasks).await?;
        Ok(())
    }

    fn get_server_name(&self) -> String {
        "idxdata".to_string()
    }
}

/// Build a set of whitelisted auth tokens. Invalid tokens are ignored.
pub fn build_auth_token_set(whitelisted_auth_tokens: Vec<String>) -> HashSet<MetadataValue<Ascii>> {
    whitelisted_auth_tokens
        .into_iter()
        .map(|token| token.parse::<MetadataValue<Ascii>>())
        .filter_map(Result::ok)
        .collect::<HashSet<_>>()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = ServerArgs::parse();
    args.run::<IndexerGrpcDataServiceConfig>().await
}
