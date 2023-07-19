// Copyright © Aptos Foundation

use crate::metrics::VERIFICATION_ERROR_COUNT;
use anyhow::{bail, ensure};
use aptos_indexer_grpc_utils::{
    config::IndexerGrpcFileStoreConfig,
    constants::BLOB_STORAGE_SIZE,
    file_store_operator::{
        FileStoreOperator, GcsFileStoreOperator, LocalFileStoreOperator, TransactionsFile,
    },
};
use aptos_protos::transaction::v1::Transaction;
use prost::Message;

pub struct FileStorageVerifier {
    pub file_store_config: IndexerGrpcFileStoreConfig,
    pub chain_id: u64,
}

impl FileStorageVerifier {
    pub fn new(file_store_config: IndexerGrpcFileStoreConfig, chain_id: u64) -> Self {
        Self {
            file_store_config,
            chain_id,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let mut file_store_operator: Box<dyn FileStoreOperator> = match &self.file_store_config {
            IndexerGrpcFileStoreConfig::GcsFileStore(gcs_file_store) => {
                Box::new(GcsFileStoreOperator::new(
                    gcs_file_store.gcs_file_store_bucket_name.clone(),
                    gcs_file_store
                        .gcs_file_store_service_account_key_path
                        .clone(),
                ))
            },
            IndexerGrpcFileStoreConfig::LocalFileStore(local_file_store) => Box::new(
                LocalFileStoreOperator::new(local_file_store.local_file_store_path.clone()),
            ),
        };
        // Verify the existence of the storage bucket.
        file_store_operator.verify_storage_bucket_existence().await;
        // Get or create verification metadata file.
        let verification_metadata = file_store_operator
            .get_or_create_verification_metadata(self.chain_id)
            .await?;
        let file_store_metadata = file_store_operator
            .get_file_store_metadata()
            .await
            .ok_or(anyhow::anyhow!("File Store metadata does not exist"))?;
        ensure!(
            file_store_metadata.chain_id == self.chain_id,
            "Chain ID mismatch"
        );
        let mut next_version_to_verify = verification_metadata.next_version_to_verify;
        let mut next_version_to_store = file_store_metadata.version;

        loop {
            if next_version_to_verify > next_version_to_store {
                VERIFICATION_ERROR_COUNT.inc();
                bail!("Next version to verify is greater than current head version, which is impossible.");
            }

            if next_version_to_verify == next_version_to_store {
                // Update the metadata in a minute and retry.
                std::thread::sleep(std::time::Duration::from_secs(60));
                tracing::info!(
                    "Retrying verification at version {}",
                    next_version_to_verify
                );
                let file_store_metadata = file_store_operator
                    .get_file_store_metadata()
                    .await
                    .ok_or(anyhow::anyhow!("File Store metadata does not exist"))?;
                next_version_to_store = file_store_metadata.version;
                continue;
            }

            // Verify the next version.
            let txn_file: TransactionsFile = file_store_operator
                .get_raw_transactions(next_version_to_verify)
                .await?;
            if txn_file.starting_version != next_version_to_verify {
                VERIFICATION_ERROR_COUNT.inc();
                bail!("Starting version of transaction file {} does not match with next version to verify {}.",
                    txn_file.starting_version, next_version_to_verify);
            }

            if txn_file.transactions.len() != BLOB_STORAGE_SIZE {
                VERIFICATION_ERROR_COUNT.inc();
                bail!(
                    "File size is not {} but {} actually",
                    BLOB_STORAGE_SIZE,
                    txn_file.transactions.len()
                );
            }
            for (index, txn) in txn_file.transactions.iter().enumerate() {
                let txn_bytes = base64::decode(txn)?;
                let txn: Transaction = Transaction::decode(&*txn_bytes)?;
                if txn.version != txn_file.starting_version + index as u64 {
                    VERIFICATION_ERROR_COUNT.inc();
                    bail!(
                        "Transaction version {} does not match with starting version {}.",
                        txn.version,
                        txn_file.starting_version + index as u64
                    );
                }
            }
            tracing::info!("Verified transaction version {}", next_version_to_verify);
            next_version_to_verify += BLOB_STORAGE_SIZE as u64;
            file_store_operator
                .update_verification_metadata(self.chain_id, next_version_to_verify)
                .await?;
        }
    }
}
