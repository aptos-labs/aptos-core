// Copyright © Aptos Foundation

use crate::{
    access_trait::{AccessMetadata, StorageReadError, StorageReadStatus, StorageTransactionRead},
    FileMetadata, SERVICE_TYPE,
};
use anyhow::Context;
use aptos_indexer_grpc_utils::{
    counters::IndexerGrpcStep,
    storage::{FileEntry, FileEntryKey, StorageFormat},
};
use aptos_protos::{indexer::v1::TransactionsInStorage, transaction::v1::Transaction};
use google_cloud_storage::{
    client::{Client, ClientConfig},
    http::{
        objects::{download::Range, get::GetObjectRequest},
        Error,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

const GCS_STORAGE_NAME: &str = "Google Cloud Storage";
const METADATA_FILE_NAME: &str = "metadata.json";
const SERVICE_ACCOUNT_ENV_VAR: &str = "SERVICE_ACCOUNT";
// Avoid reading metadata file too often and use stale metadata instead.
const METADATA_FILE_MAX_STALENESS_IN_SECS: u64 = 30; // 30 seconds.

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GcsClientConfig {
    bucket_name: String,
    service_account_path: String,
    #[serde(default = "default_storage_format")]
    storage_format: StorageFormat,
}

fn default_storage_format() -> StorageFormat {
    StorageFormat::JsonBase64UncompressedProto
}

pub type GcsClient = GcsInternalClient<google_cloud_storage::client::Client>;

impl GcsClient {
    pub async fn new(config: GcsClientConfig) -> anyhow::Result<Self> {
        std::env::set_var(SERVICE_ACCOUNT_ENV_VAR, config.service_account_path.clone());
        let gcs_config_res = ClientConfig::default()
            .with_auth()
            .await
            .context("Failed to create GCS client.");
        let gcs_config = gcs_config_res?;
        let client = Client::new(gcs_config);
        GcsInternalClient::new_with_client(config.bucket_name, config.storage_format, client).await
    }
}

// TODO: fix the broken client is copied to new connection.

#[derive(Clone)]
pub struct GcsInternalClient<T: GcsClientTrait> {
    // Bucket name.
    pub bucket_name: String,
    latest_metadata: Arc<Mutex<FileMetadata>>,
    latest_metadata_timestamp: Arc<Mutex<Option<std::time::Instant>>>,
    pub gcs_client: T,
    storage_format: StorageFormat,
}

impl<T: GcsClientTrait + Sync + Send + Clone> GcsInternalClient<T> {
    pub async fn new_with_client(
        bucket_name: String,
        storage_format: StorageFormat,
        gcs_client: T,
    ) -> anyhow::Result<Self> {
        let res = Self {
            bucket_name,
            latest_metadata: Arc::new(Mutex::new(FileMetadata::default())),
            latest_metadata_timestamp: Arc::new(Mutex::new(None)),
            gcs_client,
            storage_format,
        };
        Ok(res)
    }

    async fn refresh_metadata_if_needed(&self) -> Result<(), StorageReadError> {
        let now = std::time::Instant::now();
        {
            let latest_metadata_timestamp = self.latest_metadata_timestamp.lock().unwrap();
            if let Some(timestamp) = *latest_metadata_timestamp {
                if now.duration_since(timestamp).as_secs() < METADATA_FILE_MAX_STALENESS_IN_SECS {
                    // The metadata is fresh enough.
                    return Ok(());
                }
            }
        }
        // Hack: GCS connection might be broken, so we need to create a new client.
        // We crash the client for now.
        let file_bytes = self
            .gcs_client
            .patched_download_object(
                &GetObjectRequest {
                    bucket: self.bucket_name.clone(),
                    object: METADATA_FILE_NAME.to_string(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
            .expect("Failed to download metadata file.");
        let metadata = FileMetadata::from(file_bytes);
        {
            let mut latest_metadata = self.latest_metadata.lock().unwrap();
            *latest_metadata = metadata;
            let mut latest_metadata_timestamp = self.latest_metadata_timestamp.lock().unwrap();
            *latest_metadata_timestamp = Some(now);
        }
        Ok(())
    }
}

impl From<google_cloud_storage::http::Error> for StorageReadError {
    fn from(err: google_cloud_storage::http::Error) -> Self {
        match err {
            Error::HttpClient(e) => StorageReadError::TransientError(
                GCS_STORAGE_NAME,
                anyhow::Error::new(e).context("Failed to download object due to network issue."),
            ),
            Error::Response(e) => match e.is_retriable() {
                true => StorageReadError::TransientError(
                    GCS_STORAGE_NAME,
                    anyhow::Error::new(e).context("Failed to download object; it's transient."),
                ),
                false => StorageReadError::PermenantError(
                    GCS_STORAGE_NAME,
                    anyhow::Error::new(e).context("Failed to download object; it's permernant."),
                ),
            },
            Error::TokenSource(e) => StorageReadError::PermenantError(
                GCS_STORAGE_NAME,
                anyhow::anyhow!(e.to_string())
                    .context("Failed to download object; authenication/token error."),
            ),
        }
    }
}

#[async_trait::async_trait]
impl<T: GcsClientTrait + Sync + Send + Clone> StorageTransactionRead for GcsInternalClient<T> {
    async fn get_transactions(
        &self,
        batch_starting_version: u64,
        _size_hint: Option<usize>,
    ) -> Result<StorageReadStatus, StorageReadError> {
        let start_time = std::time::Instant::now();
        let file_name = FileEntryKey::new(batch_starting_version, self.storage_format).to_string();
        let result = self
            .gcs_client
            .patched_download_object(
                &GetObjectRequest {
                    bucket: self.bucket_name.clone(),
                    object: file_name.clone(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await;
        // Note: as the last resort of the storages, if the file cannot be found, it means
        // the transactions are not available yet and it'll be available in the future.
        let file = match result {
            Err(Error::Response(e)) if e.code == 404 => {
                return Ok(StorageReadStatus::NotAvailableYet)
            },
            Err(e) => Err(e)?,
            _ => result?,
        };

        tracing::info!(
            duration_in_secs = start_time.elapsed().as_secs_f64(),
            start_version = batch_starting_version,
            service = SERVICE_TYPE,
            step = IndexerGrpcStep::FilestoreFetchedTxns.get_step(),
            "{}",
            IndexerGrpcStep::FilestoreFetchedTxns.get_label(),
        );

        let file_entry = match self.storage_format {
            StorageFormat::JsonBase64UncompressedProto => {
                FileEntry::JsonBase64UncompressedProto(file)
            },
            StorageFormat::Bz2CompressedProto => FileEntry::Bz2CompressedProto(file),
            StorageFormat::GzipCompressionProto => FileEntry::GzipCompressionProto(file),
            _ => panic!("Unsupported storage format: {:?}", self.storage_format),
        };
        let transactions_file: TransactionsInStorage = file_entry
            .try_into()
            .expect("Failed to parse transactions file.");
        let transactions = transactions_file
            .transactions
            .into_iter()
            .filter(|x| x.version >= batch_starting_version)
            .collect::<Vec<Transaction>>();

        tracing::info!(
            duration_in_secs = start_time.elapsed().as_secs_f64(),
            start_version = batch_starting_version,
            service = SERVICE_TYPE,
            step = IndexerGrpcStep::FilestoreDecodedTxns.get_step(),
            "{}",
            IndexerGrpcStep::FilestoreDecodedTxns.get_label(),
        );

        Ok(StorageReadStatus::Ok(transactions))
    }

    async fn get_metadata(&self) -> Result<AccessMetadata, StorageReadError> {
        self.refresh_metadata_if_needed().await?;
        let mut access_metadata = AccessMetadata::default();
        {
            let latest_metadata = self.latest_metadata.lock().unwrap();
            access_metadata.chain_id = latest_metadata.chain_id;
            access_metadata.next_version = latest_metadata.version;
        }
        Ok(access_metadata)
    }
}

#[async_trait::async_trait]
pub trait GcsClientTrait: Send + Sync + Clone {
    async fn patched_download_object(
        &self,
        request: &GetObjectRequest,
        range: &Range,
    ) -> Result<Vec<u8>, Error>;
}

#[async_trait::async_trait]
impl GcsClientTrait for google_cloud_storage::client::Client {
    async fn patched_download_object(
        &self,
        request: &GetObjectRequest,
        range: &Range,
    ) -> Result<Vec<u8>, Error> {
        self.download_object(request, range).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_indexer_grpc_utils::storage::TransactionsLegacyFile;
    use aptos_protos::transaction::v1::Transaction;
    use prost::Message;
    use std::sync::atomic::{AtomicU64, Ordering};
    #[derive(Debug)]
    pub(crate) struct MockGcsClient {
        // Transactions to be returned.
        pub resps: Vec<Vec<u8>>,
        pub reqs: Vec<GetObjectRequest>,
        pub index: AtomicU64,
    }
    impl Clone for MockGcsClient {
        fn clone(&self) -> Self {
            MockGcsClient {
                resps: self.resps.clone(),
                reqs: self.reqs.clone(),
                index: AtomicU64::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl GcsClientTrait for MockGcsClient {
        async fn patched_download_object(
            &self,
            request: &GetObjectRequest,
            _range: &Range,
        ) -> Result<Vec<u8>, Error> {
            let index = self.index.fetch_add(1, Ordering::SeqCst) as usize;
            assert_eq!(self.reqs[index].object, request.object);
            assert_eq!(self.reqs[index].bucket, request.bucket);
            Ok(self.resps[index].clone())
        }
    }
    #[tokio::test]
    async fn test_get_transactions() {
        let mut transactions = Vec::new();
        for i in 0..1000 {
            let transaction = Transaction {
                version: i,
                ..Transaction::default()
            };
            transactions.push(transaction);
        }

        let serialized_transactions = serde_json::to_vec(&TransactionsLegacyFile {
            starting_version: 0,
            transactions_in_base64: transactions
                .iter()
                .map(|x| {
                    let mut buf = Vec::new();
                    x.encode(&mut buf).unwrap();
                    base64::encode(buf)
                })
                .collect::<Vec<String>>(),
        })
        .unwrap();

        let mock_gcs_client = MockGcsClient {
            resps: vec![serialized_transactions],
            reqs: vec![GetObjectRequest {
                object: "files/0.json".to_string(),
                bucket: "test1".to_string(),
                ..Default::default()
            }],
            index: AtomicU64::new(0),
        };
        let gcs_client = GcsInternalClient::new_with_client(
            "test1".to_string(),
            StorageFormat::JsonBase64UncompressedProto,
            mock_gcs_client,
        )
        .await
        .unwrap();

        let get_transactions_resp = gcs_client.get_transactions(0, None).await.unwrap();

        assert_eq!(get_transactions_resp, StorageReadStatus::Ok(transactions));
    }

    #[tokio::test]
    async fn test_get_transactions_with_partial() {
        let mut transactions = Vec::new();
        for i in 0..1000 {
            let transaction = Transaction {
                version: i,
                ..Transaction::default()
            };
            transactions.push(transaction);
        }

        let serialized_transactions = serde_json::to_vec(&TransactionsLegacyFile {
            starting_version: 0,
            transactions_in_base64: transactions
                .iter()
                .map(|x| {
                    let mut buf = Vec::new();
                    x.encode(&mut buf).unwrap();
                    base64::encode(buf)
                })
                .collect::<Vec<String>>(),
        })
        .unwrap();

        let mock_gcs_client = MockGcsClient {
            resps: vec![serialized_transactions],
            reqs: vec![GetObjectRequest {
                object: "files/0.json".to_string(),
                bucket: "test2".to_string(),
                ..Default::default()
            }],
            index: AtomicU64::new(0),
        };
        let gcs_client = GcsInternalClient::new_with_client(
            "test2".to_string(),
            StorageFormat::JsonBase64UncompressedProto,
            mock_gcs_client,
        )
        .await
        .unwrap();

        let get_transactions_resp = gcs_client.get_transactions(500, None).await.unwrap();
        assert_eq!(
            get_transactions_resp,
            StorageReadStatus::Ok(
                transactions
                    .into_iter()
                    .skip(500)
                    .collect::<Vec<Transaction>>()
            )
        );
    }

    #[tokio::test]
    async fn test_get_metadata() {
        let serialized_metadata = serde_json::to_vec(&FileMetadata {
            chain_id: 42,
            file_folder_size: 1000,
            version: 1000,
        })
        .unwrap();

        let mock_gcs_client = MockGcsClient {
            resps: vec![serialized_metadata],
            reqs: vec![GetObjectRequest {
                object: METADATA_FILE_NAME.to_string(),
                bucket: "test3".to_string(),
                ..Default::default()
            }],
            index: AtomicU64::new(0),
        };
        let gcs_client = GcsInternalClient::new_with_client(
            "test3".to_string(),
            StorageFormat::JsonBase64UncompressedProto,
            mock_gcs_client,
        )
        .await
        .unwrap();

        let get_metadata_resp = gcs_client.get_metadata().await.unwrap();

        assert_eq!(get_metadata_resp.chain_id, 42);
        assert_eq!(get_metadata_resp.next_version, 1000);
    }
    // TODO: add tests for GCS operation failures.
}
