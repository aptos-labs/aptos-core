// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::data_manager::DataManager;
use anyhow::Result;
use aptos_indexer_grpc_utils::{
    compression_util::{FileEntry, StorageFormat},
    config::IndexerGrpcFileStoreConfig,
    file_store_operator_v2::{BatchMetadata, FileStoreOperatorV2},
};
use aptos_protos::transaction::v1::Transaction;
use prost::Message;
use std::{sync::Arc, time::Duration};
use tracing::info;

const NUM_TXNS_PER_FOLDER: u64 = 10000;
const MAX_SIZE_PER_FILE: usize = 20 * (1 << 20);

pub(crate) struct FileStoreUploader {
    file_store_operator: FileStoreOperatorV2,
    buffer: Vec<Transaction>,
    buffer_size: usize,
    buffer_batch_metadata: BatchMetadata,
    version: u64,
}

impl FileStoreUploader {
    pub(crate) async fn new(
        chain_id: u64,
        file_store_config: IndexerGrpcFileStoreConfig,
    ) -> Result<Self> {
        let file_store = file_store_config.create_filestore().await;
        let file_store_operator =
            FileStoreOperatorV2::new(chain_id, file_store, NUM_TXNS_PER_FOLDER);

        file_store_operator.maybe_init_metadata().await?;

        let version = file_store_operator
            .get_latest_version()
            .await
            .expect("Latest version must exist.");

        Ok(Self {
            file_store_operator,
            buffer: vec![],
            buffer_size: 0,
            buffer_batch_metadata: BatchMetadata::default(),
            version,
        })
    }

    pub(crate) async fn start(&mut self, data_manager: Arc<DataManager>) -> Result<()> {
        loop {
            let transactions = data_manager
                .get_transactions_from_cache(
                    self.version,
                    MAX_SIZE_PER_FILE,
                    /*update_file_store_version=*/ true,
                )
                .await;
            let len = transactions.len();
            for transaction in transactions {
                self.buffer_and_maybe_dump_transactions_to_file(transaction)
                    .await?;
            }
            self.version += len as u64;
            if len == 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    async fn buffer_and_maybe_dump_transactions_to_file(
        &mut self,
        transaction: Transaction,
    ) -> Result<()> {
        let end_batch = (transaction.version + 1) % 10000 == 0;
        let size = transaction.encoded_len();
        self.buffer.push(transaction);
        self.buffer_size += size;
        if self.buffer_size >= MAX_SIZE_PER_FILE || end_batch {
            self.dump_transactions_to_file(end_batch).await?;
        }

        Ok(())
    }

    async fn dump_transactions_to_file(&mut self, end_batch: bool) -> Result<()> {
        let first_version = self.buffer.first().unwrap().version;
        let last_version = self.buffer.last().unwrap().version;
        // TODO(grao): This is slow, need to move to a different thread.
        let data_file = FileEntry::from_transactions(
            std::mem::take(&mut self.buffer),
            StorageFormat::Lz4CompressedProto,
        );
        let data_size = self.buffer_size;
        self.buffer_size = 0;
        let path = self.file_store_operator.get_path_for_version(first_version);
        self.buffer_batch_metadata
            .files
            .push((first_version, data_size));
        info!("Dumping transactions [{first_version}, {last_version}] to file {path:?}.");
        self.file_store_operator
            .save_raw_file(path, data_file.into_inner())
            .await?;
        if end_batch {
            let batch_metadata_path = self
                .file_store_operator
                .get_path_for_batch_metadata(first_version);
            self.file_store_operator
                .save_raw_file(
                    batch_metadata_path,
                    serde_json::to_vec(&self.buffer_batch_metadata).map_err(anyhow::Error::msg)?,
                )
                .await?;
            self.file_store_operator
                .update_file_store_metadata(last_version + 1)
                .await?;
            self.buffer_batch_metadata = BatchMetadata::default();
        }
        Ok(())
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
    }
}
