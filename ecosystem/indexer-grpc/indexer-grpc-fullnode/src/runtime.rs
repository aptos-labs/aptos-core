// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fullnode_data_service::FullnodeDataService, localnet_data_service::LocalnetDataService,
    ServiceContext,
};
use aptos_api::context::Context;
use aptos_config::config::{IndexerBackupRestoreConfig, NodeConfig};
use aptos_db_indexer_async_v2::{
    backup_restore_operator::{
        BackupRestoreOperator, GcsBackupRestoreOperator, LocalBackupRestoreOperator,
    },
    db::INDEX_ASYNC_V2_DB_NAME,
};
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
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::{ChainId, NamedChain};
use std::{net::ToSocketAddrs, sync::Arc};
use tokio::runtime::Runtime;
use tonic::{codec::CompressionEncoding, transport::Server};
use std::env;

// Default Values
pub const DEFAULT_NUM_RETRIES: usize = 3;
pub const RETRY_TIME_MILLIS: u64 = 100;

/// Creates a runtime which creates a thread pool which sets up the grpc streaming service
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: DbReaderWriter,
    mp_sender: MempoolClientSender,
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
    let backup_restore_config = match node_config.indexer_grpc.backup_restore_config.clone() {
        Some(config) => config.clone(),
        None => IndexerBackupRestoreConfig::default(),
    };
    let named_chain =
        match NamedChain::from_chain_id(&chain_id) {
            Ok(named_chain) => format!("{}", named_chain).to_lowercase(),
            Err(_err) => {
                info!("Getting chain name from not named chains");
                chain_id.id().to_string()
            },
        };
        let backup_restore_operator: Arc<Box<dyn BackupRestoreOperator>> =
        match &backup_restore_config.clone() {
            IndexerBackupRestoreConfig::GcsFileStore(gcs_file_store) => {
                Arc::new(Box::new(
                GcsBackupRestoreOperator::new(
                    gcs_file_store.gcs_bucket_name.clone() + "_" + &named_chain,
                ),
            ))},
            IndexerBackupRestoreConfig::LocalFileStore(local_file_store) => Arc::new(Box::new(
                LocalBackupRestoreOperator::new(local_file_store.local_file_store_path.clone()),
            )),
        };

    runtime.block_on(async {
        backup_restore_operator
            .verify_storage_bucket_existence()
            .await;
        let db_path = node_config
            .storage
            .get_dir_paths()
            .default_root_path()
            .join(INDEX_ASYNC_V2_DB_NAME);
        backup_restore_operator
            .restore_snapshot(chain_id.id() as u64, db_path.clone())
            .await
            .expect("Failed to restore snapshot");
        let _metadata = backup_restore_operator
            .create_default_metadata_if_absent(chain_id.id().into())
            .await;
    });

    runtime.spawn(async move {
        let context =
            Arc::new(Context::new(chain_id, db.reader.clone(), mp_sender, node_config));

        let service_context = ServiceContext {
            context: context.clone(),
            processor_task_count,
            processor_batch_size,
            output_batch_size,
        };
        // If we are here, we know indexer grpc is enabled.
        let server = FullnodeDataService {
            service_context: service_context.clone(),
            db_writer: db.writer.clone(),
            backup_restore_operator: backup_restore_operator.clone(),
        };
        let localnet_data_server = LocalnetDataService {
            service_context,
            db_writer: db.writer.clone(),
            backup_restore_operator: backup_restore_operator.clone(),
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
            .expect("Failed to build reflection service");

        let reflection_service_clone = reflection_service.clone();

        let tonic_server = Server::builder()
            .http2_keepalive_interval(Some(std::time::Duration::from_secs(60)))
            .http2_keepalive_timeout(Some(std::time::Duration::from_secs(5)))
            .add_service(reflection_service_clone);

        let router = match use_data_service_interface {
            false => tonic_server.add_service(FullnodeDataServer::new(server)),
            true => {
                let svc = RawDataServer::new(localnet_data_server)
                    .send_compressed(CompressionEncoding::Gzip)
                    .accept_compressed(CompressionEncoding::Gzip);
                tonic_server.add_service(svc)
            },
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
