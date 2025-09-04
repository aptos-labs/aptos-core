// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{IndexerGrpcManagerConfig, ServiceConfig, MAX_MESSAGE_SIZE},
    data_manager::DataManager,
    file_store_uploader::FileStoreUploader,
    metadata_manager::MetadataManager,
    metrics::IS_MASTER,
    service::GrpcManagerService,
};
use anyhow::Result;
use velor_protos::indexer::v1::grpc_manager_server::GrpcManagerServer;
use std::{sync::Arc, time::Duration};
use tokio::sync::{oneshot::channel, Mutex};
use tonic::{codec::CompressionEncoding, transport::Server};
use tracing::info;

const HTTP2_PING_INTERVAL_DURATION: Duration = Duration::from_secs(60);
const HTTP2_PING_TIMEOUT_DURATION: Duration = Duration::from_secs(10);

pub(crate) struct GrpcManager {
    chain_id: u64,
    file_store_uploader: Mutex<FileStoreUploader>,
    metadata_manager: Arc<MetadataManager>,
    data_manager: Arc<DataManager>,
    is_master: bool,
}

impl GrpcManager {
    pub(crate) async fn new(config: &IndexerGrpcManagerConfig) -> Self {
        let chain_id = config.chain_id;
        let file_store_uploader = Mutex::new(
            FileStoreUploader::new(chain_id, config.file_store_config.clone())
                .await
                .unwrap_or_else(|e| {
                    panic!(
                        "Failed to create filestore uploader, config: {:?}, error: {e:?}",
                        config.file_store_config
                    )
                }),
        );

        info!(
            chain_id = chain_id,
            "FilestoreUploader is created, config: {:?}.", config.file_store_config
        );

        let metadata_manager = Arc::new(MetadataManager::new(
            chain_id,
            config.self_advertised_address.clone(),
            config.grpc_manager_addresses.clone(),
            config.fullnode_addresses.clone(),
            if config.is_master {
                Some(config.self_advertised_address.clone())
            } else {
                None
            },
        ));

        info!(
            self_advertised_address = config.self_advertised_address,
            "MetadataManager is created, grpc_manager_addresses: {:?}, fullnode_addresses: {:?}.",
            config.grpc_manager_addresses,
            config.fullnode_addresses
        );

        let data_manager = Arc::new(
            DataManager::new(
                chain_id,
                config.file_store_config.clone(),
                config.cache_config.clone(),
                metadata_manager.clone(),
            )
            .await,
        );

        info!("DataManager is created.");
        IS_MASTER.set(config.is_master as i64);

        Self {
            chain_id,
            file_store_uploader,
            metadata_manager,
            data_manager,
            is_master: config.is_master,
        }
    }

    pub(crate) fn start(&self, service_config: &ServiceConfig) -> Result<()> {
        let service = GrpcManagerServer::new(GrpcManagerService::new(
            self.chain_id,
            self.metadata_manager.clone(),
            self.data_manager.clone(),
        ))
        .send_compressed(CompressionEncoding::Zstd)
        .accept_compressed(CompressionEncoding::Zstd)
        .max_encoding_message_size(MAX_MESSAGE_SIZE)
        .max_decoding_message_size(MAX_MESSAGE_SIZE);
        let server = Server::builder()
            .http2_keepalive_interval(Some(HTTP2_PING_INTERVAL_DURATION))
            .http2_keepalive_timeout(Some(HTTP2_PING_TIMEOUT_DURATION))
            .add_service(service);

        let (tx, rx) = channel();
        tokio_scoped::scope(|s| {
            s.spawn(async move {
                self.metadata_manager.start().await.unwrap();
            });
            s.spawn(async move { self.data_manager.start(self.is_master, rx).await });
            if self.is_master {
                s.spawn(async move {
                    self.file_store_uploader
                        .lock()
                        .await
                        .start(self.data_manager.clone(), tx)
                        .await
                        .unwrap();
                });
            }
            s.spawn(async move {
                info!("Starting GrpcManager at {}.", service_config.listen_address);
                server.serve(service_config.listen_address).await.unwrap();
            });
        });

        Ok(())
    }

    pub(crate) fn get_metadata_manager(&self) -> &MetadataManager {
        &self.metadata_manager
    }

    pub(crate) fn get_data_manager(&self) -> &DataManager {
        &self.data_manager
    }
}
