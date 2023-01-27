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

fn generate_blob_name(starting_version: u64, blob_size: u64) -> String {
    format!(
        "{}_{}.json",
        starting_version,
        starting_version + blob_size - 1
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn verify_blob_naming() {
        assert_eq!(super::generate_blob_name(0, 1000), "0_999");
        assert_eq!(
            super::generate_blob_name(100_000_000, 1000),
            "100000000_100000999"
        );
        assert_eq!(
            super::generate_blob_name(1_000_000_000, 1000),
            "1000000000_1000000999"
        );
        assert_eq!(
            super::generate_blob_name(10_000_000_000, 1000),
            "10000000000_10000000999"
        );
    }
}
