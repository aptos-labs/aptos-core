// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// The maximum content length to accept in the http body.
pub const MAX_CONTENT_LENGTH: u64 = 1024 * 1024;

/// GCP Header field for the current request's trace ID.
pub const GCP_CLOUD_TRACE_CONTEXT_HEADER: &str = "X-Cloud-Trace-Context";

/// GCP Cloud Run env variable for the current deployment revision
pub const GCP_CLOUD_RUN_REVISION_ENV: &str = "K_REVISION";
/// GCP Cloud Run env variable for service name
pub const GCP_CLOUD_RUN_SERVICE_ENV: &str = "K_SERVICE";
/// GCP Project within which this service is running.
/// This variable must be set by calling the metadata server
pub const GCP_SERVICE_PROJECT_ID: &str = "GCP_METADATA_PROJECT";
