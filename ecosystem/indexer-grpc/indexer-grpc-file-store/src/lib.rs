// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod processor;

/// Get redis address from env variable.
#[inline]
pub fn get_redis_address() -> String {
    std::env::var("REDIS_ADDRESS").expect("REDIS_ADDRESS is not set.")
}

#[inline]
pub fn get_file_store_bucket_name() -> String {
    std::env::var("FILE_STORE_BUCKET_NAME").expect("FILE_STORE_BUCKET_NAME is not set.")
}

#[inline]
pub fn get_file_store_blob_folder_name() -> String {
    std::env::var("FILE_STORE_BLOB_FOLDER_NAME").expect("FILE_STORE_BLOB_FOLDER_NAME is not set.")
}

#[inline]
pub fn get_chain_name() -> String {
    std::env::var("CHAIN_NAME").expect("CHAIN_NAME is not set.")
}
