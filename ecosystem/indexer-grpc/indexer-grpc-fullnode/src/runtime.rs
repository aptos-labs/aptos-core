// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fullnode_data_service::FullnodeDataService, localnet_data_service::LocalnetDataService,
    ServiceContext,
};
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_db_indexer::table_info_reader::TableInfoReader;
use aptos_indexer_grpc_stream_server::GrpcServerBuilder;
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
use aptos_types::chain_id::ChainId;
use std::{net::ToSocketAddrs, sync::Arc};
use tokio::runtime::Runtime;
use tonic::codec::CompressionEncoding;

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
    table_info_reader: Option<Arc<dyn TableInfoReader>>,
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

    runtime.spawn(async move {
        let context = Arc::new(Context::new(
            chain_id,
            db,
            mp_sender,
            node_config,
            table_info_reader,
        ));
        let service_context = ServiceContext {
            context: context.clone(),
            processor_task_count,
            processor_batch_size,
            output_batch_size,
        };
        // If we are here, we know indexer grpc is enabled.
        let server = FullnodeDataServer::new(FullnodeDataService {
            service_context: service_context.clone(),
        })
        .send_compressed(CompressionEncoding::Gzip)
        .accept_compressed(CompressionEncoding::Gzip);
        let localnet_data_server = RawDataServer::new(LocalnetDataService { service_context })
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip);

        let file_descriptors = &[
            INDEXER_V1_FILE_DESCRIPTOR_SET,
            TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET,
            UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET,
        ];

        let fn_server_builder = GrpcServerBuilder::new(server);
        let local_server_builder = GrpcServerBuilder::new(localnet_data_server);

        let router = match use_data_service_interface {
            false => fn_server_builder.build_router(None, file_descriptors),
            true => local_server_builder.build_router(None, file_descriptors),
        };
        // Make port into a config
        router
            .serve(address.to_socket_addrs().unwrap().next().unwrap())
            .await
            .unwrap();
        info!(address = address, "[indexer-grpc] Started GRPC server");
    });
    Some(runtime)
}
