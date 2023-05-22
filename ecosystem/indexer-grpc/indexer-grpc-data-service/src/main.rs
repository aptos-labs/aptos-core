// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_data_service::service::RawDataServerWrapper;
use aptos_indexer_grpc_server_framework::{RunnableConfig, ServerArgs};
use aptos_indexer_grpc_utils::config::IndexerGrpcFileStoreConfig;
use aptos_protos::{
    internal::fullnode::v1::FILE_DESCRIPTOR_SET as DATASTREAM_V1_FILE_DESCRIPTOR_SET,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcDataServiceConfig {
    pub server_name: String,
    pub data_service_grpc_listen_address: String,
    pub whitelisted_auth_tokens: Vec<String>,
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub redis_read_replica_address: String,
}

#[async_trait::async_trait]
impl RunnableConfig for IndexerGrpcDataServiceConfig {
    async fn run(&self) -> Result<()> {
        let grpc_address = self.data_service_grpc_listen_address.clone();

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
            .register_encoded_file_descriptor_set(DATASTREAM_V1_FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET)
            .build()
            .expect("Failed to build reflection service");

        // Add authentication interceptor.
        let server = RawDataServerWrapper::new(
            self.redis_read_replica_address.clone(),
            self.file_store_config.clone(),
        );
        let svc = aptos_protos::indexer::v1::raw_data_server::RawDataServer::new(server)
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip);
        let svc_with_interceptor = InterceptedService::new(svc, authentication_inceptor);
        Server::builder()
            .add_service(reflection_service)
            .add_service(svc_with_interceptor)
            .serve(grpc_address.to_socket_addrs().unwrap().next().unwrap())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to serve: {}", e))
    }

    fn get_server_name(&self) -> String {
        self.server_name.clone()
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
