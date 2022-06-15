// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// A collection of constants and default values for configuring telemetry components

// Environment variables
pub(crate) const ENV_APTOS_DISABLE_TELEMETRY: &str = "APTOS_DISABLE_TELEMETRY";
pub(crate) const ENV_GA_MEASUREMENT_ID: &str = "GA_MEASUREMENT_ID";
pub(crate) const ENV_GA_API_SECRET: &str = "GA_API_SECRET";

// Default Google Analytic values.
// TODO: Rotate these periodically.
pub(crate) const APTOS_GA_MEASUREMENT_ID: &str = "G-ZX4L6WPCFZ";
pub(crate) const APTOS_GA_API_SECRET: &str = "ArtslKPTTjeiMi1n-IR39g";

// Useful URLS.
// Note: the measurement protocol requires HTTPS.
// See: https://developers.google.com/analytics/devguides/collection/protocol/v1/reference#transport
pub(crate) const GA4_URL: &str = "https://www.google-analytics.com/mp/collect";
pub(crate) const HTTPBIN_URL: &str = "https://httpbin.org/ip";

// Frequencies for the various metrics and pushes
pub(crate) const NODE_CORE_METRICS_FREQ_SECS: u64 = 30; // 30 seconds
pub(crate) const NODE_NETWORK_METRICS_FREQ_SECS: u64 = 60; // 1 minute
pub(crate) const NODE_SYS_INFO_FREQ_SECS: u64 = 5 * 60; // 5 minutes
