// Copyright © Aptos Foundation

/// Maximum retry time for exponential backoff (2 sec = 3-4 retries)
pub const MAX_RETRY_TIME_SECONDS: u64 = 2;

/// Allocate 15 seconds for a HEAD request
pub const MAX_HEAD_REQUEST_RETRY_SECONDS: u64 = 15;

/// Allocate 30 seconds for downloading large JSON files
pub const MAX_JSON_REQUEST_RETRY_SECONDS: u64 = 30;

/// Allocate 90 seconds for downloading large image files
pub const MAX_IMAGE_REQUEST_RETRY_SECONDS: u64 = 90;

/// Default maximum file size for files to be downloaded
pub const DEFAULT_MAX_FILE_SIZE_BYTES: u32 = 5_000_000;

/// Default image quality for image optimization
pub const DEFAULT_IMAGE_QUALITY: u8 = 100;

/// Default maximum image dimensions for image optimization
pub const DEFAULT_MAX_IMAGE_DIMENSIONS: u32 = 4_096;
