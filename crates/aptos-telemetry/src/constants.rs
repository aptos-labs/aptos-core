// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// A collection of constants and default values for configuring various telemetry components.

// By default, send telemetry data to Aptos Labs
// This will help with improving the Aptos ecosystem
// This should rotate occasionally
pub const APTOS_GA_MEASUREMENT_ID: &str = "G-ZX4L6WPCFZ";
pub const APTOS_GA_API_SECRET: &str = "ArtslKPTTjeiMi1n-IR39g";

pub const HTTPBIN_URL: &str = "https://httpbin.org/ip";
// measurement protocol requires HTTPS
// https://developers.google.com/analytics/devguides/collection/protocol/v1/reference#transport
pub const GA4_URL: &str = "https://www.google-analytics.com/mp/collect";

// Timeouts
pub const NETWORK_PUSH_TIME_SECS: u64 = 30;
pub const NODE_PUSH_TIME_SECS: u64 = 30;

// Metrics events
pub const APTOS_CLI_PUSH_METRICS: &str = "APTOS_CLI_PUSH_METRICS";
pub const APTOS_NETWORK_PUSH_METRICS: &str = "APTOS_NETWORK_PUSH_METRICS";
pub const APTOS_NODE_PUSH_METRICS: &str = "APTOS_NODE_PUSH_METRICS";

// Metrics names
// Environment metrics
pub const GIT_REV_METRIC: &str = "GIT_REV";
pub const IP_ADDR_METRIC: &str = "IP_ADDRESS";

// Node metrics
pub const CHAIN_ID_METRIC: &str = "CHAIN_ID";
pub const PEER_ID_METRIC: &str = "PEER_ID";
pub const SYNCED_VERSION_METRIC: &str = "SYNCED_VERSION";

// Network metrics
pub const NETWORK_ID_METRIC: &str = "NETWORK_ID";
pub const ORIGIN_METRIC: &str = "ORIGIN";
pub const PEERS_CONNECTED_METRIC: &str = "PEERS_CONNECTED";
pub const ROLE_METRIC: &str = "ROLE";
