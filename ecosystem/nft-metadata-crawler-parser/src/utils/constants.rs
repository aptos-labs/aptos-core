// Copyright © Aptos Foundation

/// Maximum file size to process in bytes
pub const MAX_FILE_SIZE_BYTES: u32 = 5000000;

/// Maximum retry time for exponential backoff (5 sec = 3-4 retries)
pub const MAX_RETRY_TIME_SECONDS: u64 = 5;
