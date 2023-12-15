// Copyright © Aptos Foundation

use anyhow::Context;
use aptos_protos::{indexer::v1::TransactionsInStorage, transaction::v1::Transaction};
use bzip2::read::{BzDecoder, BzEncoder};
use flate2::read::{GzDecoder, GzEncoder};
use prost::Message;
use ripemd::{Digest, Ripemd128};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, io::Read};

const FILE_ENTRY_TRANSACTION_COUNT: u64 = 1000;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum StorageFormat {
    Bz2CompressedProto,
    GzipCompressionProto,
    UncompressedProto,
    // Only used for legacy file format.
    // Use by cache only.
    Base64UncompressedProto,
    // Only used for legacy file format.
    // Use by file store only.
    JsonBase64UncompressedProto,
}

pub enum CacheEntry {
    Bz2CompressedProto(Vec<u8>),
    GzipCompressionProto(Vec<u8>),
    // Only used for legacy cache entry.
    Base64UncompressedProto(Vec<u8>),
}

// into_inner to get the inner Vec<u8>
impl CacheEntry {
    pub fn from_bytes(bytes: Vec<u8>, storage_format: StorageFormat) -> Self {
        match storage_format {
            StorageFormat::Bz2CompressedProto => Self::Bz2CompressedProto(bytes),
            StorageFormat::GzipCompressionProto => Self::GzipCompressionProto(bytes),
            StorageFormat::UncompressedProto => {
                panic!("UncompressedProto is not supported.")
            },
            // Legacy format.
            StorageFormat::Base64UncompressedProto => Self::Base64UncompressedProto(bytes),
            StorageFormat::JsonBase64UncompressedProto => {
                panic!("JsonBase64UncompressedProto is not supported.")
            },
        }
    }

    pub fn into_inner(self) -> Vec<u8> {
        match self {
            CacheEntry::Bz2CompressedProto(bytes) => bytes,
            CacheEntry::GzipCompressionProto(bytes) => bytes,
            CacheEntry::Base64UncompressedProto(bytes) => bytes,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            CacheEntry::Bz2CompressedProto(bytes) => bytes.len(),
            CacheEntry::GzipCompressionProto(bytes) => bytes.len(),
            CacheEntry::Base64UncompressedProto(bytes) => bytes.len(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TransactionsLegacyFile {
    /// The version of the first transaction in the blob.
    pub starting_version: u64,
    /// The transactions in the blob.
    #[serde(rename = "transactions")]
    pub transactions_in_base64: Vec<String>,
}

/// FileStoreMetadata is the metadata for the file store.
/// It's a JSON file with name: metadata.json.
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct FileStoreMetadata {
    pub chain_id: u64,
    // The size of each file folder, BLOB_STORAGE_SIZE, i.e., 1_000.
    pub file_folder_size: usize,
    // The current version of the file store.
    pub version: u64,
    // Storage format.
    #[serde(default = "default_storage_format")]
    pub storage_format: StorageFormat,
}

fn default_storage_format() -> StorageFormat {
    StorageFormat::JsonBase64UncompressedProto
}

impl FileStoreMetadata {
    pub fn new(chain_id: u64, version: u64, storage_format: StorageFormat) -> Self {
        Self {
            chain_id,
            file_folder_size: FILE_ENTRY_TRANSACTION_COUNT as usize,
            version,
            storage_format,
        }
    }
}

pub struct CacheEntryBuilder {
    // This is used to determine how to serialize the transaction.
    // Do not use in the storage; format can be inferred or configured externally.
    storage_format: StorageFormat,
    transaction: Transaction,
}

impl CacheEntryBuilder {
    pub fn new(transaction: Transaction, storage_format: StorageFormat) -> Self {
        Self {
            storage_format,
            transaction,
        }
    }
}

pub struct CacheEntryKey {
    pub version: u64,
    pub storage_format: StorageFormat,
}

impl CacheEntryKey {
    pub fn new(version: u64, storage_format: StorageFormat) -> Self {
        Self {
            version,
            storage_format,
        }
    }
}

impl ToString for CacheEntryKey {
    // Cacche key is generated based on the assumption that the naming prefix
    // doesn't have impact on the cache key.
    fn to_string(&self) -> String {
        match self.storage_format {
            StorageFormat::Bz2CompressedProto => {
                format!("bz:{}", self.version)
            },
            StorageFormat::GzipCompressionProto => {
                format!("gz:{}", self.version)
            },
            StorageFormat::UncompressedProto => {
                format!("uc:{}", self.version)
            },
            // Legacy format.
            StorageFormat::Base64UncompressedProto => {
                format!("{}", self.version)
            },
            StorageFormat::JsonBase64UncompressedProto => {
                format!("j64:{}", self.version)
            },
        }
    }
}

impl TryFrom<CacheEntry> for Transaction {
    type Error = anyhow::Error;

    fn try_from(value: CacheEntry) -> Result<Self, Self::Error> {
        match value {
            CacheEntry::Bz2CompressedProto(bytes) => {
                let mut decompressor = BzDecoder::new(&bytes[..]);
                let mut decompressed = Vec::new();
                decompressor
                    .read_to_end(&mut decompressed)
                    .context("[Bz2Compressed] Bz2 decompression failed.")?;
                let t = Transaction::decode(decompressed.as_slice())
                    .context("[Bz2Compressed] proto deserialization failed.")?;
                Ok(t)
            },
            CacheEntry::GzipCompressionProto(bytes) => {
                let mut decompressor = GzDecoder::new(&bytes[..]);
                let mut decompressed = Vec::new();
                decompressor
                    .read_to_end(&mut decompressed)
                    .context("[GzipCompressed] Gzip decompression failed.")?;
                let t = Transaction::decode(decompressed.as_slice())
                    .context("[GzipCompressed] proto deserialization failed.")?;
                Ok(t)
            },
            CacheEntry::Base64UncompressedProto(base64) => {
                let bytes: Vec<u8> = base64::decode(base64)
                    .context("[Base64Uncompressed] base64 decoding failed.")?;
                let t = Transaction::decode(bytes.as_slice())
                    .context("[Base64] proto deserialization failed.")?;
                Ok(t)
            },
        }
    }
}

impl TryFrom<CacheEntryBuilder> for CacheEntry {
    type Error = anyhow::Error;

    fn try_from(value: CacheEntryBuilder) -> Result<Self, Self::Error> {
        let mut bytes = Vec::new();
        value
            .transaction
            .encode(&mut bytes)
            .context("proto serialization failed.")?;
        match value.storage_format {
            StorageFormat::Bz2CompressedProto => {
                let mut compressed = BzEncoder::new(bytes.as_slice(), bzip2::Compression::fast());
                let mut result = Vec::new();
                compressed
                    .read_to_end(&mut result)
                    .context("Bz2 compression failed.")?;
                Ok(CacheEntry::Bz2CompressedProto(result))
            },
            StorageFormat::GzipCompressionProto => {
                let mut compressed = GzEncoder::new(bytes.as_slice(), flate2::Compression::fast());
                let mut result = Vec::new();
                compressed
                    .read_to_end(&mut result)
                    .context("Gzip compression failed.")?;
                Ok(CacheEntry::GzipCompressionProto(result))
            },
            StorageFormat::UncompressedProto => {
                anyhow::bail!("UncompressedProto is not supported.")
            },
            StorageFormat::Base64UncompressedProto => {
                let base64 = base64::encode(bytes).into_bytes();
                Ok(CacheEntry::Base64UncompressedProto(base64))
            },
            StorageFormat::JsonBase64UncompressedProto => {
                // This is fatal to see that we are using legacy file format in cache side.
                anyhow::bail!("JsonBase64UncompressedProto is not supported.")
            },
        }
    }
}

pub enum FileEntry {
    Bz2CompressedProto(Vec<u8>),
    GzipCompressionProto(Vec<u8>),
    // Only used for legacy file format.
    JsonBase64UncompressedProto(Vec<u8>),
}

impl FileEntry {
    pub fn from_bytes(bytes: Vec<u8>, storage_format: StorageFormat) -> Self {
        match storage_format {
            StorageFormat::Bz2CompressedProto => Self::Bz2CompressedProto(bytes),
            StorageFormat::GzipCompressionProto => Self::GzipCompressionProto(bytes),
            StorageFormat::UncompressedProto => {
                panic!("UncompressedProto is not supported.")
            },
            StorageFormat::Base64UncompressedProto => {
                panic!("Base64UncompressedProto is not supported.")
            },
            StorageFormat::JsonBase64UncompressedProto => Self::JsonBase64UncompressedProto(bytes),
        }
    }

    pub fn into_inner(self) -> Vec<u8> {
        match self {
            FileEntry::Bz2CompressedProto(bytes) => bytes,
            FileEntry::GzipCompressionProto(bytes) => bytes,
            FileEntry::JsonBase64UncompressedProto(bytes) => bytes,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            FileEntry::Bz2CompressedProto(bytes) => bytes.len(),
            FileEntry::GzipCompressionProto(bytes) => bytes.len(),
            FileEntry::JsonBase64UncompressedProto(bytes) => bytes.len(),
        }
    }
}

// FileEntry is used to build the raw file to upload to the file store.
pub struct FileEntryBuilder {
    // This is used to determine how to serialize the transaction.
    // Do not use in the storage; format can be inferred or configured externally.
    storage_format: StorageFormat,
    transactions: TransactionsInStorage,
}

impl FileEntryBuilder {
    pub fn new(transactions: Vec<Transaction>, storage_format: StorageFormat) -> Self {
        let starting_version = transactions
            .first()
            .expect("Cannot build empty file")
            .version;
        let transactions_count = transactions.len();
        if transactions_count % FILE_ENTRY_TRANSACTION_COUNT as usize != 0 {
            panic!("The number of transactions to upload has to be a multiple of FILE_ENTRY_TRANSACTION_COUNT.")
        }
        if starting_version % FILE_ENTRY_TRANSACTION_COUNT != 0 {
            panic!("Starting version has to be a multiple of FILE_ENTRY_TRANSACTION_COUNT.")
        }
        let t = TransactionsInStorage {
            starting_version: Some(transactions.first().unwrap().version),
            transactions,
        };
        Self {
            storage_format,
            transactions: t,
        }
    }
}

pub struct FileEntryKey {
    pub starting_version: u64,
    pub storage_format: StorageFormat,
}

impl FileEntryKey {
    pub fn new(version: u64, storage_format: StorageFormat) -> Self {
        let starting_version =
            version / FILE_ENTRY_TRANSACTION_COUNT * FILE_ENTRY_TRANSACTION_COUNT;
        Self {
            starting_version,
            storage_format,
        }
    }
}

impl ToString for FileEntryKey {
    // File key is generated based on the assumption that the naming prefix
    // has impact on the performance.
    fn to_string(&self) -> String {
        let mut hasher = Ripemd128::new();
        hasher.update(self.starting_version.to_string());
        let file_prefix = format!("{:x}", hasher.finalize());
        match self.storage_format {
            StorageFormat::Bz2CompressedProto => {
                format!(
                    "compressed_files/bz2/{}_{}.bin",
                    file_prefix, self.starting_version
                )
            },
            StorageFormat::GzipCompressionProto => {
                format!(
                    "compressed_files/gzip/{}_{}.bin",
                    file_prefix, self.starting_version
                )
            },
            StorageFormat::UncompressedProto => {
                format!(
                    "uncompressed_files/uncompressed_proto/{}.bin",
                    self.starting_version
                )
            },
            // Legacy format.
            StorageFormat::Base64UncompressedProto => {
                format!(
                    "uncompressed_files/base64_uncompressed_proto/{}.txt",
                    self.starting_version
                )
            },
            StorageFormat::JsonBase64UncompressedProto => {
                format!("files/{}.json", self.starting_version)
            },
        }
    }
}

impl TryFrom<FileEntry> for TransactionsInStorage {
    type Error = anyhow::Error;

    fn try_from(value: FileEntry) -> Result<Self, Self::Error> {
        match value {
            FileEntry::Bz2CompressedProto(bytes) => {
                let mut decompressor = BzDecoder::new(&bytes[..]);
                let mut decompressed = Vec::new();
                decompressor
                    .read_to_end(&mut decompressed)
                    .context("[Bz2Compressed] Bz2 decompression failed.")?;
                let t = TransactionsInStorage::decode(decompressed.as_slice())
                    .context("[Bz2Compressed] proto deserialization failed.")?;
                Ok(t)
            },
            FileEntry::GzipCompressionProto(bytes) => {
                let mut decompressor = GzDecoder::new(&bytes[..]);
                let mut decompressed = Vec::new();
                decompressor
                    .read_to_end(&mut decompressed)
                    .context("[GzipCompressed] Gzip decompression failed.")?;
                let t = TransactionsInStorage::decode(decompressed.as_slice())
                    .context("[GzipCompressed] proto deserialization failed.")?;
                Ok(t)
            },
            FileEntry::JsonBase64UncompressedProto(bytes) => {
                let file: TransactionsLegacyFile = serde_json::from_slice(bytes.as_slice())
                    .context("[JsonBase64Uncompressed] json deserialization failed.")?;
                let transactions = file
                    .transactions_in_base64
                    .into_iter()
                    .map(|base64| {
                        let bytes: Vec<u8> = base64::decode(base64)
                            .context("[Base64Uncompressed] base64 decoding failed.")?;
                        let t = Transaction::decode(bytes.as_slice())
                            .context("[Base64] proto deserialization failed.")?;
                        Ok(t)
                    })
                    .collect::<Result<Vec<Transaction>, anyhow::Error>>()?;
                Ok(TransactionsInStorage {
                    starting_version: Some(file.starting_version),
                    transactions,
                })
            },
        }
    }
}

impl TryFrom<FileEntryBuilder> for FileEntry {
    type Error = anyhow::Error;

    fn try_from(value: FileEntryBuilder) -> Result<Self, Self::Error> {
        let mut bytes = Vec::new();
        value
            .transactions
            .encode(&mut bytes)
            .context("proto serialization failed.")?;
        match value.storage_format {
            StorageFormat::Bz2CompressedProto => {
                let mut compressed = BzEncoder::new(bytes.as_slice(), bzip2::Compression::fast());
                let mut result = Vec::new();
                compressed
                    .read_to_end(&mut result)
                    .context("Bz2 compression failed.")?;
                Ok(FileEntry::Bz2CompressedProto(result))
            },
            StorageFormat::GzipCompressionProto => {
                let mut compressed = GzEncoder::new(bytes.as_slice(), flate2::Compression::fast());
                let mut result = Vec::new();
                compressed
                    .read_to_end(&mut result)
                    .context("Gzip compression failed.")?;
                Ok(FileEntry::GzipCompressionProto(result))
            },
            StorageFormat::UncompressedProto => {
                anyhow::bail!("UncompressedProto is not supported.")
            },
            StorageFormat::Base64UncompressedProto => {
                anyhow::bail!("Base64UncompressedProto is not supported.")
            },
            StorageFormat::JsonBase64UncompressedProto => {
                let transactions_in_base64 = value
                    .transactions
                    .transactions
                    .into_iter()
                    .map(|transaction| {
                        let mut bytes = Vec::new();
                        transaction
                            .encode(&mut bytes)
                            .context("proto serialization failed.")?;
                        Ok(base64::encode(bytes))
                    })
                    .collect::<Result<Vec<String>, anyhow::Error>>()?;
                let file = TransactionsLegacyFile {
                    starting_version: value
                        .transactions
                        .starting_version
                        .context("starting version is missing from file")?,
                    transactions_in_base64,
                };
                let json = serde_json::to_vec(&file).context("json serialization failed.")?;
                Ok(FileEntry::JsonBase64UncompressedProto(json))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_builder_uncompressed_proto() {
        let transaction = Transaction {
            version: 42,
            epoch: 333,
            ..Transaction::default()
        };
        let builder = CacheEntryBuilder {
            storage_format: StorageFormat::UncompressedProto,
            transaction: transaction.clone(),
        };
        let cache_entry = CacheEntry::try_from(builder);
        assert!(cache_entry.is_err());
    }

    #[test]
    fn test_cache_entry_builder_base64_uncompressed_proto() {
        let transaction = Transaction {
            version: 42,
            epoch: 333,
            ..Transaction::default()
        };
        let builder = CacheEntryBuilder {
            storage_format: StorageFormat::Base64UncompressedProto,
            transaction: transaction.clone(),
        };
        let cache_entry = CacheEntry::try_from(builder).expect("CacheEntryBuilder failed.");
        let deserialized_transaction =
            Transaction::try_from(cache_entry).expect("CacheEntry deserialization failed.");
        assert_eq!(transaction, deserialized_transaction);
    }

    #[test]
    fn test_cache_entry_builder_bz2_compressed_proto() {
        let transaction = Transaction {
            version: 42,
            epoch: 333,
            ..Transaction::default()
        };
        let proto_size = transaction.encoded_len();
        let builder = CacheEntryBuilder {
            storage_format: StorageFormat::Bz2CompressedProto,
            transaction: transaction.clone(),
        };
        let cache_entry = CacheEntry::try_from(builder).expect("CacheEntryBuilder failed.");
        let compressed_size = cache_entry.size();
        assert!(compressed_size != proto_size);
        let deserialized_transaction =
            Transaction::try_from(cache_entry).expect("CacheEntry deserialization failed.");
        assert_eq!(transaction, deserialized_transaction);
    }

    #[test]
    fn test_cache_entry_builder_gzip_compressed_proto() {
        let transaction = Transaction {
            version: 42,
            epoch: 333,
            ..Transaction::default()
        };
        let proto_size = transaction.encoded_len();
        let builder = CacheEntryBuilder {
            storage_format: StorageFormat::GzipCompressionProto,
            transaction: transaction.clone(),
        };
        let cache_entry = CacheEntry::try_from(builder).expect("CacheEntryBuilder failed.");
        let compressed_size = cache_entry.size();
        assert!(compressed_size != proto_size);
        let deserialized_transaction =
            Transaction::try_from(cache_entry).expect("CacheEntry deserialization failed.");
        assert_eq!(transaction, deserialized_transaction);
    }

    #[test]
    fn test_cache_entry_builder_json_base64_uncompressed_proto() {
        let transaction = Transaction {
            version: 42,
            epoch: 333,
            ..Transaction::default()
        };
        let builder = CacheEntryBuilder {
            storage_format: StorageFormat::JsonBase64UncompressedProto,
            transaction: transaction.clone(),
        };
        let cache_entry = CacheEntry::try_from(builder);
        assert!(cache_entry.is_err());
    }

    #[test]
    fn test_file_entry_builder_uncompressed_proto() {
        let transactions = TransactionsInStorage {
            starting_version: Some(42),
            transactions: vec![],
        };
        let builder = FileEntryBuilder {
            storage_format: StorageFormat::UncompressedProto,
            transactions: transactions.clone(),
        };
        let file_entry = FileEntry::try_from(builder);
        assert!(file_entry.is_err());
    }

    #[test]
    fn test_file_entry_builder_base64_uncompressed_proto() {
        let transactions = TransactionsInStorage {
            starting_version: Some(42),
            transactions: vec![
                Transaction {
                    version: 42,
                    epoch: 333,
                    ..Transaction::default()
                },
                Transaction {
                    version: 43,
                    epoch: 333,
                    ..Transaction::default()
                },
            ],
        };
        let builder = FileEntryBuilder {
            storage_format: StorageFormat::Base64UncompressedProto,
            transactions: transactions.clone(),
        };
        let file_entry = FileEntry::try_from(builder);
        assert!(file_entry.is_err());
    }

    #[test]
    fn test_file_entry_builder_json_base64_uncompressed_proto() {
        let transactions = TransactionsInStorage {
            starting_version: Some(42),
            transactions: vec![
                Transaction {
                    version: 42,
                    epoch: 333,
                    ..Transaction::default()
                },
                Transaction {
                    version: 43,
                    epoch: 333,
                    ..Transaction::default()
                },
            ],
        };
        let builder = FileEntryBuilder {
            storage_format: StorageFormat::JsonBase64UncompressedProto,
            transactions: transactions.clone(),
        };
        let file_entry = FileEntry::try_from(builder).expect("FileEntryBuilder failed.");
        let deserialized_transactions =
            TransactionsInStorage::try_from(file_entry).expect("FileEntry deserialization failed.");
        assert_eq!(transactions, deserialized_transactions);
    }

    #[test]
    fn test_file_entry_builder_gzip_compressed_proto() {
        let transactions = TransactionsInStorage {
            starting_version: Some(42),
            transactions: vec![
                Transaction {
                    version: 42,
                    epoch: 333,
                    ..Transaction::default()
                },
                Transaction {
                    version: 43,
                    epoch: 333,
                    ..Transaction::default()
                },
            ],
        };
        let proto_size = transactions.encoded_len();
        let builder = FileEntryBuilder {
            storage_format: StorageFormat::GzipCompressionProto,
            transactions: transactions.clone(),
        };
        let file_entry = FileEntry::try_from(builder).expect("FileEntryBuilder failed.");
        let compressed_size = file_entry.size();
        assert!(compressed_size != proto_size);
        let deserialized_transactions =
            TransactionsInStorage::try_from(file_entry).expect("FileEntry deserialization failed.");
        assert_eq!(transactions, deserialized_transactions);
    }

    #[test]
    fn test_file_entry_builder_bz2_compressed_proto() {
        let transactions = TransactionsInStorage {
            starting_version: Some(42),
            transactions: vec![
                Transaction {
                    version: 42,
                    epoch: 333,
                    ..Transaction::default()
                },
                Transaction {
                    version: 43,
                    epoch: 333,
                    ..Transaction::default()
                },
            ],
        };
        let proto_size = transactions.encoded_len();
        let builder = FileEntryBuilder {
            storage_format: StorageFormat::Bz2CompressedProto,
            transactions: transactions.clone(),
        };
        let file_entry = FileEntry::try_from(builder).expect("FileEntryBuilder failed.");
        let compressed_size = file_entry.size();
        assert!(compressed_size != proto_size);
        let deserialized_transactions =
            TransactionsInStorage::try_from(file_entry).expect("FileEntry deserialization failed.");
        assert_eq!(transactions, deserialized_transactions);
    }

    // Below are cache key generation veerification tests.
    #[test]
    fn test_cache_entry_key_to_string_bz2_compressed_proto() {
        assert_eq!(
            CacheEntryKey::new(42, StorageFormat::Bz2CompressedProto).to_string(),
            "bz:42"
        );
    }
    #[test]
    fn test_cache_entry_key_to_string_gzip_compressed_proto() {
        assert_eq!(
            CacheEntryKey::new(42, StorageFormat::GzipCompressionProto).to_string(),
            "gz:42"
        );
    }
    #[test]
    fn test_cache_entry_key_to_string_uncompressed_proto() {
        assert_eq!(
            CacheEntryKey::new(42, StorageFormat::UncompressedProto).to_string(),
            "uc:42"
        );
    }
    #[test]
    fn test_cache_entry_key_to_string_base64_uncompressed_proto() {
        assert_eq!(
            CacheEntryKey::new(42, StorageFormat::Base64UncompressedProto).to_string(),
            "42"
        );
    }
    #[test]
    fn test_cache_entry_key_to_string_json_base64_uncompressed_proto() {
        assert_eq!(
            CacheEntryKey::new(42, StorageFormat::JsonBase64UncompressedProto).to_string(),
            "j64:42"
        );
    }

    #[test]
    fn test_file_entry_key_to_string_bz2_compressed_proto() {
        assert_eq!(
            FileEntryKey::new(42, StorageFormat::Bz2CompressedProto).to_string(),
            "compressed_files/bz2/3d1bff1ba654ca5fdb6ac1370533d876_0.bin"
        );
    }

    #[test]
    fn test_file_entry_key_to_string_gzip_compressed_proto() {
        assert_eq!(
            FileEntryKey::new(42, StorageFormat::GzipCompressionProto).to_string(),
            "compressed_files/gzip/3d1bff1ba654ca5fdb6ac1370533d876_0.bin"
        );
    }

    #[test]
    fn test_file_entry_key_to_string_uncompressed_proto() {
        assert_eq!(
            FileEntryKey::new(42, StorageFormat::UncompressedProto).to_string(),
            "uncompressed_files/uncompressed_proto/0.bin"
        );
    }

    #[test]
    fn test_file_entry_key_to_string_base64_uncompressed_proto() {
        assert_eq!(
            FileEntryKey::new(42, StorageFormat::Base64UncompressedProto).to_string(),
            "uncompressed_files/base64_uncompressed_proto/0.txt"
        );
    }

    #[test]
    fn test_file_entry_key_to_string_json_base64_uncompressed_proto() {
        assert_eq!(
            FileEntryKey::new(42, StorageFormat::JsonBase64UncompressedProto).to_string(),
            "files/0.json"
        );
    }

    #[test]
    fn test_new_format_not_break_existing_metadata() {
        let file_metadata_serialized_json = r#"{
            "chain_id": 1,
            "file_folder_size": 1000,
            "version": 1
        }"#;

        let file_metadata: FileStoreMetadata = serde_json::from_str(file_metadata_serialized_json)
            .expect("FileStoreMetadata deserialization failed.");

        assert_eq!(
            file_metadata.storage_format,
            StorageFormat::JsonBase64UncompressedProto
        );
        assert_eq!(file_metadata.chain_id, 1);
        assert_eq!(file_metadata.file_folder_size, 1000);
    }

    #[test]
    fn test_new_format_can_be_parse() {
        let file_metadata_serialized_json = r#"{
            "chain_id": 1,
            "file_folder_size": 1000,
            "version": 1,
            "storage_format": "Bz2CompressedProto"
        }"#;

        let file_metadata: FileStoreMetadata = serde_json::from_str(file_metadata_serialized_json)
            .expect("FileStoreMetadata deserialization failed.");

        assert_eq!(
            file_metadata.storage_format,
            StorageFormat::Bz2CompressedProto
        );
        assert_eq!(file_metadata.chain_id, 1);
        assert_eq!(file_metadata.file_folder_size, 1000);
    }
}
