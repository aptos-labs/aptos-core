// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::default_file_storage_format;
use aptos_protos::{indexer::v1::TransactionsInStorage, transaction::v1::Transaction};
use lz4::{Decoder, EncoderBuilder};
use prost::Message;
use ripemd::{Digest, Ripemd128};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub const FILE_ENTRY_TRANSACTION_COUNT: u64 = 1000;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum StorageFormat {
    Lz4CompressedProto,
    // Only used for legacy file format.
    // Use by cache only.
    Base64UncompressedProto,
    // Only used for legacy file format.
    // Use by file store only.
    JsonBase64UncompressedProto,
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
    // Storage format; backward compatible.
    #[serde(default = "default_file_storage_format")]
    pub storage_format: StorageFormat,
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

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        serde_json::from_slice(bytes.as_slice())
            .expect("FileStoreMetadata json deserialization failed.")
    }

    pub fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).expect("FileStoreMetadata json serialization failed.")
    }
}

#[derive(Debug)]
pub enum CacheEntry {
    Lz4CompressionProto(Vec<u8>),
    // Only used for legacy cache entry.
    Base64UncompressedProto(Vec<u8>),
}

impl CacheEntry {
    pub fn new(bytes: Vec<u8>, storage_format: StorageFormat) -> Self {
        match storage_format {
            StorageFormat::Lz4CompressedProto => Self::Lz4CompressionProto(bytes),
            // Legacy format.
            StorageFormat::Base64UncompressedProto => Self::Base64UncompressedProto(bytes),
            StorageFormat::JsonBase64UncompressedProto => {
                panic!("JsonBase64UncompressedProto is not supported.")
            },
        }
    }

    pub fn into_inner(self) -> Vec<u8> {
        match self {
            CacheEntry::Lz4CompressionProto(bytes) => bytes,
            CacheEntry::Base64UncompressedProto(bytes) => bytes,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            CacheEntry::Lz4CompressionProto(bytes) => bytes.len(),
            CacheEntry::Base64UncompressedProto(bytes) => bytes.len(),
        }
    }

    pub fn from_transaction(transaction: Transaction, storage_format: StorageFormat) -> Self {
        let mut bytes = Vec::new();
        transaction
            .encode(&mut bytes)
            .expect("proto serialization failed.");
        match storage_format {
            StorageFormat::Lz4CompressedProto => {
                let mut compressed = EncoderBuilder::new()
                    .level(4)
                    .build(Vec::new())
                    .expect("Lz4 compression failed.");
                compressed
                    .write_all(&bytes)
                    .expect("Lz4 compression failed.");
                CacheEntry::Lz4CompressionProto(compressed.finish().0)
            },
            StorageFormat::Base64UncompressedProto => {
                let base64 = base64::encode(bytes).into_bytes();
                CacheEntry::Base64UncompressedProto(base64)
            },
            StorageFormat::JsonBase64UncompressedProto => {
                // This is fatal to see that we are using legacy file format in cache side.
                panic!("JsonBase64UncompressedProto is not supported in cache.")
            },
        }
    }

    pub fn build_key(version: u64, storage_format: StorageFormat) -> String {
        match storage_format {
            StorageFormat::Lz4CompressedProto => {
                format!("l4:{}", version)
            },
            StorageFormat::Base64UncompressedProto => {
                format!("{}", version)
            },
            StorageFormat::JsonBase64UncompressedProto => {
                // This is fatal to see that we are using legacy file format in cache side.
                panic!("JsonBase64UncompressedProto is not supported in cache.")
            },
        }
    }

    pub fn into_transaction(self) -> Transaction {
        match self {
            CacheEntry::Lz4CompressionProto(bytes) => {
                let mut decompressor = Decoder::new(&bytes[..]).expect("Lz4 decompression failed.");
                let mut decompressed = Vec::new();
                decompressor
                    .read_to_end(&mut decompressed)
                    .expect("Lz4 decompression failed.");
                let res = Transaction::decode(decompressed.as_slice())
                    .expect("proto deserialization failed.");
                res
            },
            CacheEntry::Base64UncompressedProto(bytes) => {
                let bytes: Vec<u8> = base64::decode(bytes).expect("base64 decoding failed.");
                Transaction::decode(bytes.as_slice()).expect("proto deserialization failed.")
            },
        }
    }
}

pub enum FileEntry {
    Lz4CompressionProto(Vec<u8>),
    // Only used for legacy file format.
    JsonBase64UncompressedProto(Vec<u8>),
}

impl FileEntry {
    pub fn new(bytes: Vec<u8>, storage_format: StorageFormat) -> Self {
        match storage_format {
            StorageFormat::Lz4CompressedProto => Self::Lz4CompressionProto(bytes),
            StorageFormat::Base64UncompressedProto => {
                panic!("Base64UncompressedProto is not supported.")
            },
            StorageFormat::JsonBase64UncompressedProto => Self::JsonBase64UncompressedProto(bytes),
        }
    }

    pub fn into_inner(self) -> Vec<u8> {
        match self {
            FileEntry::Lz4CompressionProto(bytes) => bytes,
            FileEntry::JsonBase64UncompressedProto(bytes) => bytes,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            FileEntry::Lz4CompressionProto(bytes) => bytes.len(),
            FileEntry::JsonBase64UncompressedProto(bytes) => bytes.len(),
        }
    }

    pub fn from_transactions(
        transactions: Vec<Transaction>,
        storage_format: StorageFormat,
    ) -> Self {
        let mut bytes = Vec::new();
        let starting_version = transactions
            .first()
            .expect("Cannot build empty file")
            .version;
        /*
        let transactions_count = transactions.len();
        if transactions_count % FILE_ENTRY_TRANSACTION_COUNT as usize != 0 {
            panic!("The number of transactions to upload has to be a multiple of FILE_ENTRY_TRANSACTION_COUNT.")
        }
        if starting_version % FILE_ENTRY_TRANSACTION_COUNT != 0 {
            panic!("Starting version has to be a multiple of FILE_ENTRY_TRANSACTION_COUNT.")
        }*/
        match storage_format {
            StorageFormat::Lz4CompressedProto => {
                let t = TransactionsInStorage {
                    starting_version: Some(transactions.first().unwrap().version),
                    transactions,
                };
                t.encode(&mut bytes).expect("proto serialization failed.");
                let mut compressed = EncoderBuilder::new()
                    .level(4)
                    .build(Vec::new())
                    .expect("Lz4 compression failed.");
                compressed
                    .write_all(&bytes)
                    .expect("Lz4 compression failed.");
                FileEntry::Lz4CompressionProto(compressed.finish().0)
            },
            StorageFormat::Base64UncompressedProto => {
                panic!("Base64UncompressedProto is not supported.")
            },
            StorageFormat::JsonBase64UncompressedProto => {
                let transactions_in_base64 = transactions
                    .into_iter()
                    .map(|transaction| {
                        let mut bytes = Vec::new();
                        transaction
                            .encode(&mut bytes)
                            .expect("proto serialization failed.");
                        base64::encode(bytes)
                    })
                    .collect::<Vec<String>>();
                let file = TransactionsLegacyFile {
                    starting_version,
                    transactions_in_base64,
                };
                let json = serde_json::to_vec(&file).expect("json serialization failed.");
                FileEntry::JsonBase64UncompressedProto(json)
            },
        }
    }

    pub fn build_key(version: u64, storage_format: StorageFormat) -> String {
        let starting_version =
            version / FILE_ENTRY_TRANSACTION_COUNT * FILE_ENTRY_TRANSACTION_COUNT;
        let mut hasher = Ripemd128::new();
        hasher.update(starting_version.to_string());
        let file_prefix = format!("{:x}", hasher.finalize());
        match storage_format {
            StorageFormat::Lz4CompressedProto => {
                format!(
                    "compressed_files/lz4/{}_{}.bin",
                    file_prefix, starting_version
                )
            },
            StorageFormat::JsonBase64UncompressedProto => {
                format!("files/{}.json", starting_version)
            },
            StorageFormat::Base64UncompressedProto => {
                panic!("Base64UncompressedProto is not supported.")
            },
        }
    }

    pub fn into_transactions_in_storage(self) -> TransactionsInStorage {
        match self {
            FileEntry::Lz4CompressionProto(bytes) => {
                let mut decompressor = Decoder::new(&bytes[..]).expect("Lz4 decompression failed.");
                let mut decompressed = Vec::new();
                decompressor
                    .read_to_end(&mut decompressed)
                    .expect("Lz4 decompression failed.");
                TransactionsInStorage::decode(decompressed.as_slice())
                    .expect("proto deserialization failed.")
            },
            FileEntry::JsonBase64UncompressedProto(bytes) => {
                let file: TransactionsLegacyFile =
                    serde_json::from_slice(bytes.as_slice()).expect("json deserialization failed.");
                let transactions = file
                    .transactions_in_base64
                    .into_iter()
                    .map(|base64| {
                        let bytes: Vec<u8> =
                            base64::decode(base64).expect("base64 decoding failed.");
                        Transaction::decode(bytes.as_slice())
                            .expect("proto deserialization failed.")
                    })
                    .collect::<Vec<Transaction>>();
                TransactionsInStorage {
                    starting_version: Some(file.starting_version),
                    transactions,
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_builder_base64_uncompressed_proto() {
        let transaction = Transaction {
            version: 42,
            epoch: 333,
            ..Transaction::default()
        };
        let transaction_clone = transaction.clone();
        let transaction_size = transaction.encoded_len();
        let cache_entry =
            CacheEntry::from_transaction(transaction, StorageFormat::Base64UncompressedProto);
        // Make sure data is compressed.
        assert_ne!(cache_entry.size(), transaction_size);
        let deserialized_transaction = cache_entry.into_transaction();
        assert_eq!(transaction_clone, deserialized_transaction);
    }

    #[test]
    fn test_cache_entry_builder_lz4_compressed_proto() {
        let transaction = Transaction {
            version: 42,
            epoch: 333,
            ..Transaction::default()
        };
        let transaction_clone = transaction.clone();
        let proto_size = transaction.encoded_len();
        let cache_entry =
            CacheEntry::from_transaction(transaction, StorageFormat::Lz4CompressedProto);
        let compressed_size = cache_entry.size();
        assert!(compressed_size != proto_size);
        let deserialized_transaction = cache_entry.into_transaction();
        assert_eq!(transaction_clone, deserialized_transaction);
    }

    #[test]
    #[should_panic]
    fn test_cache_entry_builder_json_base64_uncompressed_proto() {
        let transaction = Transaction {
            version: 42,
            epoch: 333,
            ..Transaction::default()
        };
        let _cache_entry =
            CacheEntry::from_transaction(transaction, StorageFormat::JsonBase64UncompressedProto);
    }

    #[test]
    #[should_panic]
    fn test_file_entry_builder_base64_uncompressed_proto_not_supported() {
        let transactions = (1000..2000)
            .map(|version| Transaction {
                version,
                epoch: 333,
                ..Transaction::default()
            })
            .collect::<Vec<Transaction>>();
        let _file_entry =
            FileEntry::from_transactions(transactions, StorageFormat::Base64UncompressedProto);
    }

    #[test]
    fn test_file_entry_builder_json_base64_uncompressed_proto() {
        let transactions = (1000..2000)
            .map(|version| Transaction {
                version,
                epoch: 333,
                ..Transaction::default()
            })
            .collect::<Vec<Transaction>>();
        let file_entry = FileEntry::from_transactions(
            transactions.clone(),
            StorageFormat::JsonBase64UncompressedProto,
        );
        let deserialized_transactions = file_entry.into_transactions_in_storage();
        for (i, transaction) in transactions.iter().enumerate() {
            assert_eq!(transaction, &deserialized_transactions.transactions[i]);
        }
    }

    #[test]
    fn test_file_entry_builder_lz4_compressed_proto() {
        let transactions = (1000..2000)
            .map(|version| Transaction {
                version,
                epoch: 333,
                ..Transaction::default()
            })
            .collect::<Vec<Transaction>>();
        let transactions_in_storage = TransactionsInStorage {
            starting_version: Some(1000),
            transactions: transactions.clone(),
        };
        let transactions_in_storage_size = transactions_in_storage.encoded_len();
        let file_entry =
            FileEntry::from_transactions(transactions.clone(), StorageFormat::Lz4CompressedProto);
        assert_ne!(file_entry.size(), transactions_in_storage_size);
        let deserialized_transactions = file_entry.into_transactions_in_storage();
        for (i, transaction) in transactions.iter().enumerate() {
            assert_eq!(transaction, &deserialized_transactions.transactions[i]);
        }
    }

    #[test]
    fn test_cache_entry_key_to_string_lz4_compressed_proto() {
        assert_eq!(
            CacheEntry::build_key(42, StorageFormat::Lz4CompressedProto),
            "l4:42"
        );
    }

    #[test]
    fn test_cache_entry_key_to_string_base64_uncompressed_proto() {
        assert_eq!(
            CacheEntry::build_key(42, StorageFormat::Base64UncompressedProto),
            "42"
        );
    }

    #[test]
    #[should_panic]
    fn test_cache_entry_key_to_string_json_base64_uncompressed_proto() {
        let _key = CacheEntry::build_key(42, StorageFormat::JsonBase64UncompressedProto);
    }

    #[test]
    fn test_file_entry_key_to_string_lz4_compressed_proto() {
        assert_eq!(
            FileEntry::build_key(42, StorageFormat::Lz4CompressedProto),
            "compressed_files/lz4/3d1bff1ba654ca5fdb6ac1370533d876_0.bin"
        );
    }

    #[test]
    #[should_panic]
    fn test_file_entry_key_to_string_base64_uncompressed_proto() {
        let _key = FileEntry::build_key(42, StorageFormat::Base64UncompressedProto);
    }

    #[test]
    fn test_file_entry_key_to_string_json_base64_uncompressed_proto() {
        assert_eq!(
            FileEntry::build_key(42, StorageFormat::JsonBase64UncompressedProto),
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
            "storage_format": "Lz4CompressedProto"
        }"#;

        let file_metadata: FileStoreMetadata = serde_json::from_str(file_metadata_serialized_json)
            .expect("FileStoreMetadata deserialization failed.");

        assert_eq!(
            file_metadata.storage_format,
            StorageFormat::Lz4CompressedProto
        );
        assert_eq!(file_metadata.chain_id, 1);
        assert_eq!(file_metadata.file_folder_size, 1000);
    }
}
