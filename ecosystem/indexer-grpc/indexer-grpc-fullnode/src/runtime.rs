// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fullnode_data_service::FullnodeDataService, localnet_data_service::LocalnetDataService,
    ServiceContext,
};
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use aptos_mempool::MempoolClientSender;
use aptos_protos::{
    indexer::v1::raw_data_server::RawDataServer,
    internal::fullnode::v1::fullnode_data_server::FullnodeDataServer,
};
use aptos_storage_interface::DbReader;
use aptos_types::chain_id::ChainId;
use std::{net::ToSocketAddrs, sync::Arc};
use tokio::runtime::Runtime;
use tonic::transport::Server;

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
) -> Option<Runtime> {
    if !config.indexer_grpc.enabled {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("indexer-grpc".to_string(), None);

    let node_config = config.clone();

    // We have defaults for these so they should all return something nonnull so unwrap is safe here
    let processor_task_count = node_config.indexer_grpc.processor_task_count.unwrap();
    let processor_batch_size = node_config.indexer_grpc.processor_batch_size.unwrap();
    let output_batch_size = node_config.indexer_grpc.output_batch_size.unwrap();
    let address = node_config.indexer_grpc.address.clone().unwrap();

    let dev_mode_enabled = node_config.indexer_grpc.dev_mode.unwrap_or(false);

    runtime.spawn(async move {
        let context = Arc::new(Context::new(chain_id, db, mp_sender, node_config));
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

        let mut tonic_server = Server::builder()
            .http2_keepalive_interval(Some(std::time::Duration::from_secs(60)))
            .http2_keepalive_timeout(Some(std::time::Duration::from_secs(5)));

        let router = match dev_mode_enabled {
            false => tonic_server.add_service(FullnodeDataServer::new(server)),
            true => tonic_server.add_service(RawDataServer::new(localnet_data_server)),
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
