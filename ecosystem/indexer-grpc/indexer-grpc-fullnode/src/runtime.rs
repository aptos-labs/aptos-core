// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fullnode_data_service::FullnodeDataService, localnet_data_service::LocalnetDataService,
    localnet_websocket_service::{websocket_events_handler, websocket_transactions_handler},
    ServiceContext,
};
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use aptos_mempool::MempoolClientSender;
use aptos_protos::{
    indexer::v1::{
        raw_data_server::RawDataServer, FILE_DESCRIPTOR_SET as INDEXER_V1_FILE_DESCRIPTOR_SET,
    },
    internal::fullnode::v1::fullnode_data_server::FullnodeDataServer,
    transaction::v1::FILE_DESCRIPTOR_SET as TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET,
    util::timestamp::FILE_DESCRIPTOR_SET as UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET,
};
use aptos_storage_interface::DbReader;
use aptos_types::{chain_id::ChainId, indexer::indexer_db_reader::IndexerReader};
use axum::{routing::get, Router};
use futures::channel::oneshot;
use std::sync::Arc;
use tokio::{net::TcpListener, runtime::Runtime};
use tonic::{
    codec::CompressionEncoding,
    transport::{server::TcpIncoming, Server},
};

// Default Values
pub const DEFAULT_NUM_RETRIES: usize = 3;
pub const RETRY_TIME_MILLIS: u64 = 100;

/// Creates a runtime which creates a thread pool which sets up the grpc streaming service
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
    indexer_reader: Option<Arc<dyn IndexerReader>>,
    port_tx: Option<oneshot::Sender<u16>>,
) -> Option<Runtime> {
    if !config.indexer_grpc.enabled {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("indexer-grpc".to_string(), None);

    let node_config = config.clone();

    let address = node_config.indexer_grpc.address;
    let use_data_service_interface = node_config.indexer_grpc.use_data_service_interface;
    let processor_task_count = node_config.indexer_grpc.processor_task_count;
    let processor_batch_size = node_config.indexer_grpc.processor_batch_size;
    let output_batch_size = node_config.indexer_grpc.output_batch_size;
    let websocket_enabled = node_config.indexer_grpc.websocket_enabled;
    let websocket_address = node_config.indexer_grpc.websocket_address;

    runtime.spawn(async move {
        let context = Arc::new(Context::new(
            chain_id,
            db,
            mp_sender,
            node_config,
            indexer_reader,
        ));
        let service_context = ServiceContext {
            context: context.clone(),
            processor_task_count,
            processor_batch_size,
            output_batch_size,
        };
        // If we are here, we know indexer grpc is enabled.
        let server = FullnodeDataService {
            service_context: service_context.clone(),
        };
        let localnet_data_server = LocalnetDataService { service_context };

        let reflection_service = tonic_reflection::server::Builder::configure()
            // Note: It is critical that the file descriptor set is registered for every
            // file that the top level API proto depends on recursively. If you don't,
            // compilation will still succeed but reflection will fail at runtime.
            //
            // TODO: Add a test for this / something in build.rs, this is a big footgun.
            .register_encoded_file_descriptor_set(INDEXER_V1_FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET)
            .register_encoded_file_descriptor_set(UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET)
            .build_v1()
            .expect("Failed to build reflection service");

        let reflection_service_clone = reflection_service.clone();

        let tonic_server = Server::builder()
            .http2_keepalive_interval(Some(std::time::Duration::from_secs(60)))
            .http2_keepalive_timeout(Some(std::time::Duration::from_secs(5)))
            .add_service(reflection_service_clone);

        let router = match use_data_service_interface {
            false => {
                let svc = FullnodeDataServer::new(server)
                    .send_compressed(CompressionEncoding::Zstd)
                    .accept_compressed(CompressionEncoding::Zstd)
                    .accept_compressed(CompressionEncoding::Gzip);
                tonic_server.add_service(svc)
            },
            true => {
                let svc = RawDataServer::new(localnet_data_server)
                    .send_compressed(CompressionEncoding::Zstd)
                    .accept_compressed(CompressionEncoding::Zstd)
                    .accept_compressed(CompressionEncoding::Gzip);
                tonic_server.add_service(svc)
            },
        };

        let listener = TcpListener::bind(address).await.unwrap();
        if let Some(port_tx) = port_tx {
            port_tx.send(listener.local_addr().unwrap().port()).unwrap();
        }
        let incoming = TcpIncoming::from_listener(listener, false, None).unwrap();

        // Start gRPC server
        let grpc_handle = tokio::spawn(async move {
            router.serve_with_incoming(incoming).await.unwrap();
        });

        info!(address = address, "[indexer-grpc] Started GRPC server");

        // Start WebSocket server if enabled
        if websocket_enabled {
            let service_context_ws = Arc::new(ServiceContext {
                context: context.clone(),
                processor_task_count,
                processor_batch_size,
                output_batch_size,
            });

            let ws_handle = tokio::spawn(async move {
                let app = Router::new()
                    .route("/ws/transactions", get(websocket_transactions_handler))
                    .route("/ws/events", get(websocket_events_handler))
                    .with_state(service_context_ws);

                info!(
                    websocket_address = websocket_address.to_string().as_str(),
                    "[indexer-grpc] Starting WebSocket server"
                );

                let listener = TcpListener::bind(websocket_address).await.unwrap();
                axum::serve(listener, app).await.unwrap();
            });

            info!(address = websocket_address, "[indexer-grpc] Started WebSocket server");

            // Wait for both servers
            let _ = tokio::try_join!(grpc_handle, ws_handle);
        } else {
            // Wait for just gRPC server
            let _ = grpc_handle.await;
        }
    });
    Some(runtime)
}
