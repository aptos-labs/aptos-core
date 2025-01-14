// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{IndexerGrpcManagerConfig, ServiceConfig},
    service::GrpcManagerService,
};
use anyhow::Result;
use aptos_protos::indexer::v1::grpc_manager_server::GrpcManagerServer;
use std::time::Duration;
use tonic::{codec::CompressionEncoding, transport::Server};
use tracing::info;

const HTTP2_PING_INTERVAL_DURATION: Duration = Duration::from_secs(60);
const HTTP2_PING_TIMEOUT_DURATION: Duration = Duration::from_secs(10);

pub(crate) struct GrpcManager {
    chain_id: u64,
}

impl GrpcManager {
    pub(crate) async fn new(config: &IndexerGrpcManagerConfig) -> Self {
        let chain_id = config.chain_id;

        Self { chain_id }
    }

    pub(crate) fn start(&self, service_config: &ServiceConfig) -> Result<()> {
        let service = GrpcManagerServer::new(GrpcManagerService::new(self.chain_id))
            .send_compressed(CompressionEncoding::Zstd)
            .accept_compressed(CompressionEncoding::Zstd);
        let server = Server::builder()
            .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
            .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION))
            .add_service(service);

        tokio_scoped::scope(|s| {
            s.spawn(async move {
                info!("Starting GrpcManager at {}.", service_config.listen_address);
                server.serve(service_config.listen_address).await.unwrap();
            });
        });

        Ok(())
    }
}
