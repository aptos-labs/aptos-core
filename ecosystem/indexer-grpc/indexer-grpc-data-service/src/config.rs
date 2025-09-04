// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::service::RawDataServerWrapper;
use anyhow::{bail, Result};
use velor_indexer_grpc_server_framework::RunnableConfig;
use velor_indexer_grpc_utils::{
    compression_util::StorageFormat, config::IndexerGrpcFileStoreConfig,
    in_memory_cache::InMemoryCacheConfig, types::RedisUrl,
};
use velor_protos::{
    indexer::v1::FILE_DESCRIPTOR_SET as INDEXER_V1_FILE_DESCRIPTOR_SET,
    transaction::v1::FILE_DESCRIPTOR_SET as TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET,
    util::timestamp::FILE_DESCRIPTOR_SET as UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET,
};
use velor_transaction_filter::BooleanTransactionFilter;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tonic::{codec::CompressionEncoding, transport::Server};

pub const SERVER_NAME: &str = "idxdatasvc";

// Default max response channel size.
const DEFAULT_MAX_RESPONSE_CHANNEL_SIZE: usize = 3;

// HTTP2 ping interval and timeout.
// This can help server to garbage collect dead connections.
// tonic server: https://docs.rs/tonic/latest/tonic/transport/server/struct.Server.html#method.http2_keepalive_interval
const HTTP2_PING_INTERVAL_DURATION: std::time::Duration = std::time::Duration::from_secs(60);
const HTTP2_PING_TIMEOUT_DURATION: std::time::Duration = std::time::Duration::from_secs(10);

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
    /// Deprecated: a list of auth tokens that are allowed to access the service.
    #[serde(default)]
    pub whitelisted_auth_tokens: Vec<String>,
    /// Deprecated: if set, don't check for auth tokens.
    #[serde(default)]
    pub disable_auth_check: bool,
    /// File store config.
    pub file_store_config: IndexerGrpcFileStoreConfig,
    /// Redis read replica address.
    pub redis_read_replica_address: RedisUrl,
    /// Support compressed cache data.
    #[serde(default = "IndexerGrpcDataServiceConfig::default_enable_cache_compression")]
    pub enable_cache_compression: bool,
    #[serde(default)]
    pub in_memory_cache_config: InMemoryCacheConfig,
    /// Any transaction that matches this filter will be stripped. This means we remove
    /// the payload, signature, events, and writesets from it before sending it
    /// downstream. This should only be used in an emergency situation, e.g. when txns
    /// related to a certain module are too large and are causing issues for the data
    /// service. Learn more here:
    ///
    /// https://www.notion.so/velorlabs/Runbook-c006a37259394ac2ba904d6b54d180fa?pvs=4#171c210964ec42a89574fc80154f9e85
    ///
    /// Generally you will want to start with this with an OR, and then list out
    /// separate filters that describe each type of txn we want to strip.
    #[serde(default = "IndexerGrpcDataServiceConfig::default_txns_to_strip_filter")]
    pub txns_to_strip_filter: BooleanTransactionFilter,
}

impl IndexerGrpcDataServiceConfig {
    pub fn new(
        data_service_grpc_tls_config: Option<TlsConfig>,
        data_service_grpc_non_tls_config: Option<NonTlsConfig>,
        data_service_response_channel_size: Option<usize>,
        disable_auth_check: bool,
        file_store_config: IndexerGrpcFileStoreConfig,
        redis_read_replica_address: RedisUrl,
        enable_cache_compression: bool,
        in_memory_cache_config: InMemoryCacheConfig,
        txns_to_strip_filter: BooleanTransactionFilter,
    ) -> Self {
        Self {
            data_service_grpc_tls_config,
            data_service_grpc_non_tls_config,
            data_service_response_channel_size: data_service_response_channel_size
                .unwrap_or_else(Self::default_data_service_response_channel_size),
            whitelisted_auth_tokens: vec![],
            disable_auth_check,
            file_store_config,
            redis_read_replica_address,
            enable_cache_compression,
            in_memory_cache_config,
            txns_to_strip_filter,
        }
    }

    pub const fn default_data_service_response_channel_size() -> usize {
        DEFAULT_MAX_RESPONSE_CHANNEL_SIZE
    }

    pub const fn default_enable_cache_compression() -> bool {
        false
    }

    pub fn default_txns_to_strip_filter() -> BooleanTransactionFilter {
        // This filter matches no txns.
        BooleanTransactionFilter::new_or(vec![])
    }
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcDataServiceConfig {
    fn validate(&self) -> Result<()> {
        if self.data_service_grpc_non_tls_config.is_none()
            && self.data_service_grpc_tls_config.is_none()
        {
            bail!("At least one of data_service_grpc_non_tls_config and data_service_grpc_tls_config must be set");
        }
        self.in_memory_cache_config.validate()?;
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        let reflection_service = tonic_reflection::server::Builder::configure()
            // Note: It is critical that the file descriptor set is registered for every
            // file that the top level API proto depends on recursively. If you don't,
            // compilation will still succeed but reflection will fail at runtime.
            //
            // TODO: Add a test for this / something in build.rs, this is a big footgun.
            .register_encoded_file_descriptor_set(INDEXER_V1_FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET)
            .build_v1alpha()
            .map_err(|e| anyhow::anyhow!("Failed to build reflection service: {}", e))?
            .send_compressed(CompressionEncoding::Zstd)
            .accept_compressed(CompressionEncoding::Zstd)
            .accept_compressed(CompressionEncoding::Gzip);

        let cache_storage_format: StorageFormat = if self.enable_cache_compression {
            StorageFormat::Lz4CompressedProto
        } else {
            StorageFormat::Base64UncompressedProto
        };

        println!(
            ">>>> Starting Redis connection: {:?}",
            &self.redis_read_replica_address.0
        );
        let redis_conn = redis::Client::open(self.redis_read_replica_address.0.clone())?
            .get_tokio_connection_manager()
            .await?;
        println!(">>>> Redis connection established");
        // InMemoryCache.
        let in_memory_cache =
            velor_indexer_grpc_utils::in_memory_cache::InMemoryCache::new_with_redis_connection(
                self.in_memory_cache_config.clone(),
                redis_conn,
                cache_storage_format,
            )
            .await?;
        println!(">>>> InMemoryCache established");
        // Add authentication interceptor.
        let server = RawDataServerWrapper::new(
            self.redis_read_replica_address.clone(),
            self.file_store_config.clone(),
            self.data_service_response_channel_size,
            self.txns_to_strip_filter.clone(),
            cache_storage_format,
            Arc::new(in_memory_cache),
        )?;
        let svc = velor_protos::indexer::v1::raw_data_server::RawDataServer::new(server)
            .send_compressed(CompressionEncoding::Zstd)
            .accept_compressed(CompressionEncoding::Zstd)
            .accept_compressed(CompressionEncoding::Gzip);
        println!(">>>> Starting gRPC server: {:?}", &svc);

        let svc_clone = svc.clone();
        let reflection_service_clone = reflection_service.clone();

        let mut tasks = vec![];
        if let Some(config) = &self.data_service_grpc_non_tls_config {
            let listen_address = config.data_service_grpc_listen_address;
            tracing::info!(
                grpc_address = listen_address.to_string().as_str(),
                "[data service] starting gRPC server with non-TLS."
            );
            tasks.push(tokio::spawn(async move {
                Server::builder()
                    .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
                    .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION))
                    .add_service(svc_clone)
                    .add_service(reflection_service_clone)
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
            tasks.push(tokio::spawn(async move {
                Server::builder()
                    .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
                    .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION))
                    .tls_config(tonic::transport::ServerTlsConfig::new().identity(identity))?
                    .add_service(svc)
                    .add_service(reflection_service)
                    .serve(listen_address)
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }));
        }

        futures::future::try_join_all(tasks).await?;
        Ok(())
    }

    fn get_server_name(&self) -> String {
        SERVER_NAME.to_string()
    }
}
