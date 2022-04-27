// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// A collection of constants and default values for configuring various telemetry components.

// By default, send telemetry data to Aptos Labs
// This will help with improving the Aptos ecosystem
// This should rotate occasionally
pub const APTOS_GA_MEASUREMENT_ID: &str = "G-ZX4L6WPCFZ";
pub const APTOS_GA_API_SECRET: &str = "ArtslKPTTjeiMi1n-IR39g";

pub const HTTPBIN_URL: &str = "http://httpbin.org/ip";
pub const GA4_URL: &str = "https://www.google-analytics.com/mp/collect";

// Metrics events
pub const APTOS_NODE_PUSH_METRICS: &str = "APTOS_NODE_PUSH_METRICS";
pub const APTOS_CLI_PUSH_METRICS: &str = "APTOS_CLI_PUSH_METRICS";

// Metrics names
pub const IP_ADDR_METRIC: &str = "IP_ADDRESS";
pub const GIT_REV_METRIC: &str = "GIT_REV";
pub const CHAIN_ID_METRIC: &str = "CHAIN_ID";
pub const PEER_ID_METRIC: &str = "PEER_ID";
