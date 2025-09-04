// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::file_store_operator_v2::common::{BatchMetadata, FileMetadata};
use anyhow::{ensure, Result};
use velor_protos::transaction::v1::Transaction;
use prost::Message;
use tokio::sync::mpsc::Sender;

pub struct FileStoreOperatorV2 {
    max_size_per_file: usize,
    num_txns_per_folder: u64,

    buffer: Vec<Transaction>,
    buffer_size_in_bytes: usize,
    buffer_batch_metadata: BatchMetadata,
    version: u64,
}

impl FileStoreOperatorV2 {
    pub fn new(
        max_size_per_file: usize,
        num_txns_per_folder: u64,
        version: u64,
        batch_metadata: BatchMetadata,
    ) -> Self {
        Self {
            max_size_per_file,
            num_txns_per_folder,
            buffer: vec![],
            buffer_size_in_bytes: 0,
            buffer_batch_metadata: batch_metadata,
            version,
        }
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// Buffers a transaction, if the size of the buffer exceeds the threshold, or the transaction
    /// is the last one in the batch, dump the buffer to the file store.
    pub async fn buffer_and_maybe_dump_transactions_to_file(
        &mut self,
        transaction: Transaction,
        tx: Sender<(Vec<Transaction>, BatchMetadata, bool)>,
    ) -> Result<()> {
        let end_batch = (transaction.version + 1) % self.num_txns_per_folder == 0;
        let size_bytes = transaction.encoded_len();
        ensure!(
            self.version == transaction.version,
            "Gap is found when buffering transaction, expected: {}, actual: {}",
            self.version,
            transaction.version,
        );
        self.buffer.push(transaction);
        self.buffer_size_in_bytes += size_bytes;
        self.version += 1;
        if self.buffer_size_in_bytes >= self.max_size_per_file || end_batch {
            self.dump_transactions_to_file(end_batch, tx).await?;
        }

        Ok(())
    }

    async fn dump_transactions_to_file(
        &mut self,
        end_batch: bool,
        tx: Sender<(Vec<Transaction>, BatchMetadata, bool)>,
    ) -> Result<()> {
        let transactions = std::mem::take(&mut self.buffer);
        let first_version = transactions.first().unwrap().version;
        self.buffer_batch_metadata.files.push(FileMetadata {
            first_version,
            last_version: first_version + transactions.len() as u64,
            size_bytes: self.buffer_size_in_bytes,
        });
        self.buffer_size_in_bytes = 0;

        tx.send((transactions, self.buffer_batch_metadata.clone(), end_batch))
            .await
            .map_err(anyhow::Error::msg)?;

        if end_batch {
            self.buffer_batch_metadata = BatchMetadata::default();
        }

        Ok(())
    }
}
