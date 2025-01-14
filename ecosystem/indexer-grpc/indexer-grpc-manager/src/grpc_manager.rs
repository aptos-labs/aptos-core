// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{IndexerGrpcManagerConfig, ServiceConfig},
    metadata_manager::MetadataManager,
    service::GrpcManagerService,
};
use anyhow::Result;
use aptos_protos::indexer::v1::grpc_manager_server::GrpcManagerServer;
use std::{sync::Arc, time::Duration};
use tonic::{codec::CompressionEncoding, transport::Server};
use tracing::info;

const HTTP2_PING_INTERVAL_DURATION: Duration = Duration::from_secs(60);
const HTTP2_PING_TIMEOUT_DURATION: Duration = Duration::from_secs(10);

pub(crate) struct GrpcManager {
    chain_id: u64,
    metadata_manager: Arc<MetadataManager>,
}

impl GrpcManager {
    pub(crate) async fn new(config: &IndexerGrpcManagerConfig) -> Self {
        let chain_id = config.chain_id;

        let metadata_manager = Arc::new(MetadataManager::new(
            chain_id,
            config.self_advertised_address.clone(),
            config.grpc_manager_addresses.clone(),
            config.fullnode_addresses.clone(),
        ));

        info!(
            self_advertised_address = config.self_advertised_address,
            "MetadataManager is created, grpc_manager_addresses: {:?}, fullnode_addresses: {:?}.",
            config.grpc_manager_addresses,
            config.fullnode_addresses
        );

        Self {
            chain_id,
            metadata_manager,
        }
    }

    pub(crate) fn start(&self, service_config: &ServiceConfig) -> Result<()> {
        let service = GrpcManagerServer::new(GrpcManagerService::new(
            self.chain_id,
            self.metadata_manager.clone(),
        ))
        .send_compressed(CompressionEncoding::Zstd)
        .accept_compressed(CompressionEncoding::Zstd);
        let server = Server::builder()
            .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
            .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION))
            .add_service(service);

        tokio_scoped::scope(|s| {
            s.spawn(async move {
                self.metadata_manager.start().await.unwrap();
            });
            s.spawn(async move {
                info!("Starting GrpcManager at {}.", service_config.listen_address);
                server.serve(service_config.listen_address).await.unwrap();
            });
        });

        Ok(())
    }
}
