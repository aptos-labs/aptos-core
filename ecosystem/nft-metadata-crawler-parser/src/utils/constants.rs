// Copyright © Aptos Foundation

/// Maximum retry time for exponential backoff (5 sec = 3-4 retries)
pub const MAX_RETRY_TIME_SECONDS: u64 = 5;

/// Allocate 30 seconds for downloading large JSON files
pub const MAX_JSON_REQUEST_RETRY_SECONDS: u64 = 30;

/// Allocate 180 seconds for downloading large image files
pub const MAX_IMAGE_REQUEST_RETRY_SECONDS: u64 = 180;
