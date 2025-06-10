// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A collection of constants and default values for configuring telemetry components

// Environment variables
pub(crate) const ENV_APTOS_DISABLE_TELEMETRY: &str = "APTOS_DISABLE_TELEMETRY";
pub(crate) const ENV_APTOS_FORCE_ENABLE_TELEMETRY: &str = "APTOS_FORCE_ENABLE_TELEMETRY";
pub(crate) const ENV_APTOS_DISABLE_TELEMETRY_PUSH_METRICS: &str =
    "APTOS_DISABLE_TELEMETRY_PUSH_METRICS";
pub(crate) const ENV_APTOS_DISABLE_TELEMETRY_PUSH_LOGS: &str = "APTOS_DISABLE_TELEMETRY_PUSH_LOGS";
pub(crate) const ENV_APTOS_DISABLE_TELEMETRY_PUSH_EVENTS: &str =
    "APTOS_DISABLE_TELEMETRY_PUSH_EVENTS";
pub(crate) const ENV_APTOS_DISABLE_PROMETHEUS_NODE_METRICS: &str =
    "APTOS_DISABLE_PROMETHEUS_NODE_METRICS";
pub(crate) const ENV_APTOS_DISABLE_LOG_ENV_POLLING: &str = "APTOS_DISABLE_LOG_ENV_POLLING";

pub(crate) const ENV_GA_MEASUREMENT_ID: &str = "GA_MEASUREMENT_ID";
pub(crate) const ENV_GA_API_SECRET: &str = "GA_API_SECRET";
pub(crate) const ENV_TELEMETRY_SERVICE_URL: &str = "TELEMETRY_SERVICE_URL";

// Default Google Analytic values.
// TODO: Rotate these periodically.
pub(crate) const APTOS_GA_MEASUREMENT_ID: &str = "G-ZX4L6WPCFZ";
pub(crate) const APTOS_GA_API_SECRET: &str = "ArtslKPTTjeiMi1n-IR39g";

// Useful URLS.
// Note: the measurement protocol requires HTTPS.
// See: https://developers.google.com/analytics/devguides/collection/protocol/v1/reference#transport
pub(crate) const GA4_URL: &str = "https://www.google-analytics.com/mp/collect";
pub(crate) const HTTPBIN_URL: &str = "https://httpbin.org/ip";
pub(crate) const TELEMETRY_SERVICE_URL: &str = "http://localhost:9464";
pub(crate) const MAINNET_TELEMETRY_SERVICE_URL: &str = "http://localhost:9464";

// Frequencies for the various metrics and pushes
pub(crate) const NODE_BUILD_INFO_FREQ_SECS: u64 = 60 * 60; // 60 minutes
pub(crate) const NODE_CORE_METRICS_FREQ_SECS: u64 = 30; // 30 seconds
pub(crate) const NODE_NETWORK_METRICS_FREQ_SECS: u64 = 60; // 1 minute
pub(crate) const NODE_SYS_INFO_FREQ_SECS: u64 = 5 * 60; // 5 minutes
pub(crate) const NODE_CONFIG_FREQ_SECS: u64 = 60 * 60; // 60 minutes

// TODO: consider making this interval configurable
pub(crate) const PROMETHEUS_PUSH_METRICS_FREQ_SECS: u64 = 15; // 15 seconds
pub(crate) const CHAIN_ACCESS_CHECK_FREQ_SECS: u64 = 30 * 60; // 30 minutes
pub(crate) const LOG_ENV_POLL_FREQ_SECS: u64 = 5 * 60; // 5 minutes
