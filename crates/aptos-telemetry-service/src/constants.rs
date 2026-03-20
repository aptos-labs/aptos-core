// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// The maximum content length to accept in the http body (compressed).
pub const MAX_CONTENT_LENGTH: u64 = 1024 * 1024;

/// The maximum decompressed content length to accept (10x compressed limit).
/// Protects against decompression bomb attacks.
pub const MAX_DECOMPRESSED_LENGTH: usize = 10 * 1024 * 1024;

/// GCP Header field for the current request's trace ID.
pub const GCP_CLOUD_TRACE_CONTEXT_HEADER: &str = "X-Cloud-Trace-Context";

/// GCP Cloud Run env variable for the current deployment revision
pub const GCP_CLOUD_RUN_REVISION_ENV: &str = "K_REVISION";
/// GCP Cloud Run env variable for service name
pub const GCP_CLOUD_RUN_SERVICE_ENV: &str = "K_SERVICE";
/// GCP Project within which this service is running.
/// This variable must be set by calling the metadata server
pub const GCP_SERVICE_PROJECT_ID_ENV: &str = "GCP_METADATA_PROJECT_ID";
/// Environment variable with the container identifier for this cloud run revision
/// This variable must be set by calling the metadata server
pub const GCP_CLOUD_RUN_INSTANCE_ID_ENV: &str = "GCP_CLOUD_RUN_INSTANCE_ID";
/// The IP address key
pub const IP_ADDRESS_KEY: &str = "IP_ADDRESS";
