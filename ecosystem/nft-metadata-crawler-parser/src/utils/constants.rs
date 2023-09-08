// Copyright © Aptos Foundation

/// Maximum retry time for exponential backoff (5 sec = 3-4 retries)
pub const MAX_RETRY_TIME_SECONDS: u64 = 5;

/// Allocate 5 seconds for a HEAD request
pub const MAX_HEAD_REQUEST_RETRY_SECONDS: u64 = 5;

/// Allocate 30 seconds for downloading large JSON files
pub const MAX_JSON_REQUEST_RETRY_SECONDS: u64 = 30;

/// Allocate 180 seconds for downloading large image files
pub const MAX_IMAGE_REQUEST_RETRY_SECONDS: u64 = 180;

/// Skip URIs that contain the following strings
pub const URI_SKIP_LIST: [&str; 5] = [
    "aptoslabs.com/nft_images/aptos-zero",
    "svg.souffl3.com",
    "api.apt.store",
    "aptosnames.com",
    "aptos.dev",
];
