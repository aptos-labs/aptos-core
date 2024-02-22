// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{service::RawDataServerWrapper, utils::GrpcServerBuilder};
use anyhow::{bail, Result};
use aptos_indexer_grpc_server_framework::RunnableConfig;
use aptos_indexer_grpc_utils::{
    compression_util::StorageFormat, config::IndexerGrpcFileStoreConfig, types::RedisUrl,
};
use aptos_protos::{
    indexer::v1::FILE_DESCRIPTOR_SET as INDEXER_V1_FILE_DESCRIPTOR_SET,
    transaction::v1::FILE_DESCRIPTOR_SET as TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET,
    util::timestamp::FILE_DESCRIPTOR_SET as UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, net::SocketAddr};
use tonic::{
    codec::CompressionEncoding,
    metadata::{Ascii, MetadataValue},
    Request, Status,
};

pub const SERVER_NAME: &str = "idxdatasvc";

// Default max response channel size.
const DEFAULT_MAX_RESPONSE_CHANNEL_SIZE: usize = 3;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TlsConfig {
    /// The address for the TLS GRPC server to listen on.
    pub data_service_grpc_listen_address: SocketAddr,
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonTlsConfig {
    /// The address for the TLS GRPC server to listen on.
    pub data_service_grpc_listen_address: SocketAddr,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcDataServiceConfig {
    /// If given, we will run a server that uses TLS.
    pub data_service_grpc_tls_config: Option<TlsConfig>,
    /// If given, we will run a server that does not use TLS.
    pub data_service_grpc_non_tls_config: Option<NonTlsConfig>,
    /// The size of the response channel that response can be buffered.
    #[serde(default = "IndexerGrpcDataServiceConfig::default_data_service_response_channel_size")]
    pub data_service_response_channel_size: usize,
    /// A list of auth tokens that are allowed to access the service.
    pub whitelisted_auth_tokens: Vec<String>,
    /// If set, don't check for auth tokens.
    #[serde(default)]
    pub disable_auth_check: bool,
    /// File store config.
    pub file_store_config: IndexerGrpcFileStoreConfig,
    /// Redis read replica address.
    pub redis_read_replica_address: RedisUrl,
    /// Support compressed cache data.
    #[serde(default = "IndexerGrpcDataServiceConfig::default_enable_cache_compression")]
    pub enable_cache_compression: bool,
}

impl IndexerGrpcDataServiceConfig {
    pub fn new(
        data_service_grpc_tls_config: Option<TlsConfig>,
        data_service_grpc_non_tls_config: Option<NonTlsConfig>,
        data_service_response_channel_size: Option<usize>,
        whitelisted_auth_tokens: Vec<String>,
        disable_auth_check: bool,
        file_store_config: IndexerGrpcFileStoreConfig,
        redis_read_replica_address: RedisUrl,
        enable_cache_compression: bool,
    ) -> Self {
        Self {
            data_service_grpc_tls_config,
            data_service_grpc_non_tls_config,
            data_service_response_channel_size: data_service_response_channel_size
                .unwrap_or_else(Self::default_data_service_response_channel_size),
            whitelisted_auth_tokens,
            disable_auth_check,
            file_store_config,
            redis_read_replica_address,
            enable_cache_compression,
        }
    }

    pub const fn default_data_service_response_channel_size() -> usize {
        DEFAULT_MAX_RESPONSE_CHANNEL_SIZE
    }

    pub const fn default_enable_cache_compression() -> bool {
        false
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcDataServiceConfig {
    fn validate(&self) -> Result<()> {
        if self.disable_auth_check && !self.whitelisted_auth_tokens.is_empty() {
            bail!("disable_auth_check is set but whitelisted_auth_tokens is not empty");
        }
        if !self.disable_auth_check && self.whitelisted_auth_tokens.is_empty() {
            bail!("disable_auth_check is not set but whitelisted_auth_tokens is empty");
        }
        if self.data_service_grpc_non_tls_config.is_none()
            && self.data_service_grpc_tls_config.is_none()
        {
            bail!("At least one of data_service_grpc_non_tls_config and data_service_grpc_tls_config must be set");
        }
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        let token_set = build_auth_token_set(self.whitelisted_auth_tokens.clone());
        let disable_auth_check = self.disable_auth_check;
        let authentication_inceptor =
            move |req: Request<()>| -> std::result::Result<Request<()>, Status> {
                if disable_auth_check {
                    return std::result::Result::Ok(req);
                }
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

        let cache_storage_format: StorageFormat = if self.enable_cache_compression {
            StorageFormat::GzipCompressedProto
        } else {
            StorageFormat::Base64UncompressedProto
        };
        let server = RawDataServerWrapper::new(
            self.redis_read_replica_address.clone(),
            self.file_store_config.clone(),
            self.data_service_response_channel_size,
            cache_storage_format,
        )?;
        let svc = aptos_protos::indexer::v1::raw_data_server::RawDataServer::new(server)
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip);

        let grpc_server_builder = GrpcServerBuilder::new(svc, authentication_inceptor);

        let mut tasks = vec![];
        if let Some(config) = &self.data_service_grpc_non_tls_config {
            let listen_address = config.data_service_grpc_listen_address;
            tracing::info!(
                grpc_address = listen_address.to_string().as_str(),
                "[data service] starting gRPC server with non-TLS."
            );
            let router = grpc_server_builder.build_router(None, &[
                INDEXER_V1_FILE_DESCRIPTOR_SET,
                TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET,
                UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET,
            ]);
            tasks.push(tokio::spawn(async move {
                router
                    .serve(listen_address)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }));
        }
        if let Some(config) = &self.data_service_grpc_tls_config {
            let listen_address = config.data_service_grpc_listen_address;
            let cert = tokio::fs::read(config.cert_path.clone()).await?;
            let key = tokio::fs::read(config.key_path.clone()).await?;
            let identity = tonic::transport::Identity::from_pem(cert, key);
            tracing::info!(
                grpc_address = listen_address.to_string().as_str(),
                "[Data Service] Starting gRPC server with TLS."
            );
            let router = grpc_server_builder.build_router(
                Some(tonic::transport::ServerTlsConfig::new().identity(identity)),
                &[
                    INDEXER_V1_FILE_DESCRIPTOR_SET,
                    TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET,
                    UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET,
                ],
            );
            tasks.push(tokio::spawn(async move {
                router
                    .serve(listen_address)
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
        SERVER_NAME.to_string()
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
