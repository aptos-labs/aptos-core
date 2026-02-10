// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    config::{
        config_sanitizer::ConfigSanitizer, gas_estimation_config::GasEstimationConfig,
        node_config_loader::NodeType, Error, NodeConfig, MAX_RECEIVING_BLOCK_TXNS,
    },
    utils,
};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiConfig {
    /// Enables the v1 REST API endpoint (Poem-based).
    ///
    /// Requires the `api-v1` Cargo feature to be compiled in.
    /// If the feature is absent, this flag is ignored and a warning is logged.
    ///
    /// Set to `false` with `api_v2.enabled: true` for v2-only mode.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Address for the REST API to listen on. Set to 0.0.0.0:port to allow all inbound connections.
    pub address: SocketAddr,
    /// Path to a local TLS certificate to enable HTTPS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,
    /// Path to a local TLS key to enable HTTPS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,
    /// A maximum limit to the body of a POST request in bytes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_length_limit: Option<u64>,
    /// Enables failpoints for error testing
    #[serde(default = "default_disabled")]
    pub failpoints_enabled: bool,
    /// Enables JSON output of APIs that support it
    #[serde(default = "default_enabled")]
    pub json_output_enabled: bool,
    /// Enables BCS output of APIs that support it
    #[serde(default = "default_enabled")]
    pub bcs_output_enabled: bool,
    /// Enables compression middleware for API responses
    #[serde(default = "default_enabled")]
    pub compression_enabled: bool,
    /// Enables encode submission API
    #[serde(default = "default_enabled")]
    pub encode_submission_enabled: bool,
    /// Enables transaction submission APIs
    #[serde(default = "default_enabled")]
    pub transaction_submission_enabled: bool,
    /// Enables transaction simulation
    #[serde(default = "default_enabled")]
    pub transaction_simulation_enabled: bool,
    /// Maximum number of transactions that can be sent with the Batch submit API
    pub max_submit_transaction_batch_size: usize,
    /// Maximum page size for transaction paginated APIs
    pub max_transactions_page_size: u16,
    /// Maximum page size for block transaction APIs
    pub max_block_transactions_page_size: u16,
    /// Maximum page size for event paginated APIs
    pub max_events_page_size: u16,
    /// Maximum page size for resource paginated APIs
    pub max_account_resources_page_size: u16,
    /// Maximum page size for module paginated APIs
    pub max_account_modules_page_size: u16,
    /// Maximum gas unit limit for view functions
    ///
    /// This limits the execution length of a view function to the given gas used.
    pub max_gas_view_function: u64,
    /// Optional: Maximum number of worker threads for the API.
    ///
    /// If not set, `runtime_worker_multiplier` will multiply times the number of CPU cores on the machine
    pub max_runtime_workers: Option<usize>,
    /// Multiplier for number of worker threads with number of CPU cores
    ///
    /// If `max_runtime_workers` is set, this is ignored
    pub runtime_worker_multiplier: usize,
    /// Configs for computing unit gas price estimation
    pub gas_estimation: GasEstimationConfig,
    /// Periodically call gas estimation
    pub periodic_gas_estimation_ms: Option<u64>,
    /// Configuration to filter view function requests.
    pub view_filter: ViewFilter,
    /// Periodically log stats for view function and simulate transaction usage
    pub periodic_function_stats_sec: Option<u64>,
    /// The time wait_by_hash will wait before returning 404.
    pub wait_by_hash_timeout_ms: u64,
    /// The interval at which wait_by_hash will poll the storage for the transaction.
    pub wait_by_hash_poll_interval_ms: u64,
    /// The number of active wait_by_hash requests that can be active at any given time.
    pub wait_by_hash_max_active_connections: usize,
    /// Allow submission of encrypted transactions via the API
    pub allow_encrypted_txns_submission: bool,
}

const DEFAULT_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_REQUEST_CONTENT_LENGTH_LIMIT: u64 = 8 * 1024 * 1024; // 8 MB
pub const DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE: usize = 10;
pub const DEFAULT_MAX_PAGE_SIZE: u16 = 100;
const DEFAULT_MAX_ACCOUNT_RESOURCES_PAGE_SIZE: u16 = 9999;
const DEFAULT_MAX_ACCOUNT_MODULES_PAGE_SIZE: u16 = 9999;
const DEFAULT_MAX_VIEW_GAS: u64 = 2_000_000; // We keep this value the same as the max number of gas allowed for one single transaction defined in aptos-gas.

fn default_enabled() -> bool {
    true
}

fn default_disabled() -> bool {
    false
}

impl Default for ApiConfig {
    fn default() -> ApiConfig {
        ApiConfig {
            enabled: default_enabled(),
            address: format!("{}:{}", DEFAULT_ADDRESS, DEFAULT_PORT)
                .parse()
                .unwrap(),
            tls_cert_path: None,
            tls_key_path: None,
            content_length_limit: None,
            failpoints_enabled: default_disabled(),
            bcs_output_enabled: default_enabled(),
            json_output_enabled: default_enabled(),
            compression_enabled: default_enabled(),
            encode_submission_enabled: default_enabled(),
            transaction_submission_enabled: default_enabled(),
            transaction_simulation_enabled: default_enabled(),
            max_submit_transaction_batch_size: DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE,
            max_block_transactions_page_size: *MAX_RECEIVING_BLOCK_TXNS as u16,
            max_transactions_page_size: DEFAULT_MAX_PAGE_SIZE,
            max_events_page_size: DEFAULT_MAX_PAGE_SIZE,
            max_account_resources_page_size: DEFAULT_MAX_ACCOUNT_RESOURCES_PAGE_SIZE,
            max_account_modules_page_size: DEFAULT_MAX_ACCOUNT_MODULES_PAGE_SIZE,
            max_gas_view_function: DEFAULT_MAX_VIEW_GAS,
            max_runtime_workers: None,
            runtime_worker_multiplier: 2,
            gas_estimation: GasEstimationConfig::default(),
            periodic_gas_estimation_ms: Some(30_000),
            view_filter: ViewFilter::default(),
            periodic_function_stats_sec: Some(60),
            wait_by_hash_timeout_ms: 1_000,
            wait_by_hash_poll_interval_ms: 20,
            wait_by_hash_max_active_connections: 100,
            allow_encrypted_txns_submission: false,
        }
    }
}

impl ApiConfig {
    pub fn randomize_ports(&mut self) {
        self.address.set_port(utils::get_available_port());
    }

    pub fn content_length_limit(&self) -> u64 {
        match self.content_length_limit {
            Some(v) => v,
            None => DEFAULT_REQUEST_CONTENT_LENGTH_LIMIT,
        }
    }
}

impl ConfigSanitizer for ApiConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let api_config = &node_config.api;

        // If the API is disabled, we don't need to do anything
        if !api_config.enabled {
            return Ok(());
        }

        // Verify that failpoints are not enabled in mainnet
        if let Some(chain_id) = chain_id {
            if chain_id.is_mainnet() && api_config.failpoints_enabled {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "Failpoints are not supported on mainnet nodes!".into(),
                ));
            }
        }

        // Validate basic runtime properties
        if api_config.max_runtime_workers.is_none() && api_config.runtime_worker_multiplier == 0 {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "runtime_worker_multiplier must be greater than 0!".into(),
            ));
        }

        // Sanitize the gas estimation config
        GasEstimationConfig::sanitize(node_config, node_type, chain_id)?;

        Ok(())
    }
}

/// Configuration for the v2 REST API (Axum-based).
///
/// ## Feature Flag Interaction
///
/// The v2 API supports compile-time feature flags in addition to runtime
/// configuration. A feature is only active when **both** the Cargo feature
/// is compiled in **and** the runtime flag is enabled.
///
/// | Cargo Feature        | Runtime Flag              | Effect                         |
/// |----------------------|---------------------------|--------------------------------|
/// | `api-v2`             | `enabled: true`           | v2 REST API available          |
/// | `api-v2-websocket`   | `websocket_enabled: true` | WebSocket endpoint available   |
/// | `api-v2-sse`         | `sse_enabled: true`       | SSE endpoints available        |
///
/// If a runtime flag is `true` but the corresponding feature was not compiled in,
/// a warning is logged at startup and the flag is effectively ignored.
///
/// ## v2-Only Mode
///
/// To run the node with only the v2 API (no Poem v1):
/// - Set `api.enabled: false` and `api_v2.enabled: true` in config, OR
/// - Compile with `--no-default-features --features api-v2,api-v2-websocket,api-v2-sse`
///
/// In v2-only mode, requests to `/v1/*` will return 404.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiV2Config {
    /// Enables the v2 REST API.
    ///
    /// Requires the `api-v2` Cargo feature to be compiled in.
    /// If the feature is absent, this flag is ignored and a warning is logged.
    #[serde(default = "default_disabled")]
    pub enabled: bool,
    /// Optional separate address for the v2 API. If None, shares the v1 port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<SocketAddr>,
    /// Enables WebSocket support on v2.
    ///
    /// Requires the `api-v2-websocket` Cargo feature to be compiled in.
    #[serde(default = "default_enabled")]
    pub websocket_enabled: bool,
    /// Maximum number of concurrent WebSocket connections.
    pub websocket_max_connections: usize,
    /// Maximum subscriptions per WebSocket connection.
    pub websocket_max_subscriptions_per_conn: usize,
    /// Enables Server-Sent Events (SSE) endpoints on v2.
    ///
    /// Requires the `api-v2-sse` Cargo feature to be compiled in.
    #[serde(default = "default_enabled")]
    pub sse_enabled: bool,
    /// Enables HTTP/2 (h2c) support.
    #[serde(default = "default_enabled")]
    pub http2_enabled: bool,
    /// Maximum number of requests in a JSON-RPC batch.
    pub json_rpc_batch_max_size: usize,
    /// Optional content length limit override. If None, inherits from v1 config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_length_limit: Option<u64>,
    /// Optional maximum number of worker threads for the v2 API runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_runtime_workers: Option<usize>,
    /// Worker thread multiplier (used if max_runtime_workers is None).
    pub runtime_worker_multiplier: usize,
    /// Per-request timeout in milliseconds. Requests exceeding this duration
    /// receive a 408 Request Timeout response. Set to 0 to disable.
    pub request_timeout_ms: u64,
    /// Timeout (in milliseconds) for draining in-flight connections during
    /// graceful shutdown. After this period, remaining connections are
    /// forcibly closed. Set to 0 to shut down immediately.
    pub graceful_shutdown_timeout_ms: u64,
    /// Path to PEM-encoded TLS certificate for HTTPS. Both `tls_cert_path` and
    /// `tls_key_path` must be set to enable TLS. When TLS is enabled, ALPN
    /// negotiation supports both `h2` and `http/1.1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_cert_path: Option<String>,
    /// Path to PEM-encoded TLS private key for HTTPS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls_key_path: Option<String>,
}

impl Default for ApiV2Config {
    fn default() -> Self {
        Self {
            enabled: default_disabled(),
            address: None,
            websocket_enabled: default_enabled(),
            sse_enabled: default_enabled(),
            websocket_max_connections: 1000,
            websocket_max_subscriptions_per_conn: 10,
            http2_enabled: default_enabled(),
            json_rpc_batch_max_size: 20,
            content_length_limit: None,
            max_runtime_workers: None,
            runtime_worker_multiplier: 2,
            request_timeout_ms: 30_000,
            graceful_shutdown_timeout_ms: 30_000,
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

impl ApiV2Config {
    /// Returns true if TLS is configured (both cert and key paths are set).
    pub fn tls_enabled(&self) -> bool {
        self.tls_cert_path.is_some() && self.tls_key_path.is_some()
    }
}

impl ConfigSanitizer for ApiV2Config {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let v2 = &node_config.api_v2;

        // If v2 API is disabled, skip validation
        if !v2.enabled {
            return Ok(());
        }

        // --- Runtime worker configuration ---
        if v2.max_runtime_workers.is_none() && v2.runtime_worker_multiplier == 0 {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "runtime_worker_multiplier must be greater than 0 when max_runtime_workers is not set!".into(),
            ));
        }

        // --- TLS: both paths must be set together ---
        match (&v2.tls_cert_path, &v2.tls_key_path) {
            (Some(_), None) => {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "tls_cert_path is set but tls_key_path is missing. Both must be provided for TLS.".into(),
                ));
            },
            (None, Some(_)) => {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "tls_key_path is set but tls_cert_path is missing. Both must be provided for TLS.".into(),
                ));
            },
            _ => {},
        }

        // --- WebSocket limits ---
        if v2.websocket_enabled && v2.websocket_max_connections == 0 {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "websocket_max_connections must be greater than 0 when WebSocket is enabled!".into(),
            ));
        }
        if v2.websocket_enabled && v2.websocket_max_subscriptions_per_conn == 0 {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "websocket_max_subscriptions_per_conn must be greater than 0 when WebSocket is enabled!".into(),
            ));
        }

        // --- Batch size ---
        if v2.json_rpc_batch_max_size == 0 {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "json_rpc_batch_max_size must be greater than 0!".into(),
            ));
        }

        // --- Content length limit ---
        if let Some(limit) = v2.content_length_limit {
            if limit == 0 {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "content_length_limit must be greater than 0 when explicitly set!".into(),
                ));
            }
        }

        // --- Address conflict: v2 separate address must not equal v1 address ---
        if let Some(v2_addr) = v2.address {
            if v2_addr == node_config.api.address {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    format!(
                        "v2 address ({}) must differ from the v1 API address ({}). \
                         Remove api_v2.address to use same-port co-hosting instead.",
                        v2_addr, node_config.api.address
                    ),
                ));
            }
        }

        Ok(())
    }
}

// This is necessary because we can't import the EntryFunctionId type from the API types.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ViewFunctionId {
    pub address: AccountAddress,
    pub module: String,
    pub function_name: String,
}

// We just accept Strings here because we can't import EntryFunctionId. We sanitize
// the values later.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewFilter {
    /// Allowlist of functions. If a function is not found here, the API will refuse to
    /// service the view / simulation request.
    Allowlist(Vec<ViewFunctionId>),
    /// Blocklist of functions. If a function is found here, the API will refuse to
    /// service the view / simulation request.
    Blocklist(Vec<ViewFunctionId>),
}

impl Default for ViewFilter {
    fn default() -> Self {
        ViewFilter::Blocklist(vec![])
    }
}

impl ViewFilter {
    /// Returns true if the given function is allowed by the filter.
    pub fn allows(&self, address: &AccountAddress, module: &str, function: &str) -> bool {
        match self {
            ViewFilter::Allowlist(ids) => ids.iter().any(|id| {
                &id.address == address && id.module == module && id.function_name == function
            }),
            ViewFilter::Blocklist(ids) => !ids.iter().any(|id| {
                &id.address == address && id.module == module && id.function_name == function
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_disabled_api() {
        // Create a node config with the API disabled
        let node_config = NodeConfig {
            api: ApiConfig {
                enabled: false,
                failpoints_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it succeeds
        ApiConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet())).unwrap();
    }

    #[test]
    fn test_sanitize_failpoints_on_mainnet() {
        // Create a node config with failpoints enabled
        let node_config = NodeConfig {
            api: ApiConfig {
                enabled: true,
                failpoints_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails because
        // failpoints are not supported on mainnet.
        let error =
            ApiConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));

        // Sanitize the config for a different network and verify that it succeeds
        ApiConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::testnet())).unwrap();

        // Sanitize the config for an unknown network and verify that it succeeds
        ApiConfig::sanitize(&node_config, NodeType::Validator, None).unwrap();
    }

    #[test]
    fn test_sanitize_invalid_workers() {
        // Create a node config with failpoints enabled
        let node_config = NodeConfig {
            api: ApiConfig {
                enabled: true,
                max_runtime_workers: None,
                runtime_worker_multiplier: 0,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails because
        // the runtime worker multiplier is invalid.
        let error =
            ApiConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    // ---- ApiV2Config sanitizer tests ----

    /// Helper to create a NodeConfig with v2 API enabled and the given overrides.
    fn v2_node_config(v2: ApiV2Config) -> NodeConfig {
        NodeConfig {
            api_v2: v2,
            ..Default::default()
        }
    }

    #[test]
    fn test_v2_sanitize_disabled_skips_validation() {
        // When v2 is disabled, even invalid settings should pass.
        let cfg = v2_node_config(ApiV2Config {
            enabled: false,
            runtime_worker_multiplier: 0,
            json_rpc_batch_max_size: 0,
            ..Default::default()
        });
        ApiV2Config::sanitize(&cfg, NodeType::Validator, Some(ChainId::mainnet())).unwrap();
    }

    #[test]
    fn test_v2_sanitize_default_passes() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            ..Default::default()
        });
        ApiV2Config::sanitize(&cfg, NodeType::Validator, Some(ChainId::mainnet())).unwrap();
    }

    #[test]
    fn test_v2_sanitize_invalid_runtime_workers() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            max_runtime_workers: None,
            runtime_worker_multiplier: 0,
            ..Default::default()
        });
        let err =
            ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap_err();
        assert!(matches!(err, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_v2_sanitize_explicit_workers_bypasses_multiplier_check() {
        // When max_runtime_workers is explicitly set, multiplier doesn't matter.
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            max_runtime_workers: Some(4),
            runtime_worker_multiplier: 0,
            ..Default::default()
        });
        ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap();
    }

    #[test]
    fn test_v2_sanitize_tls_cert_without_key() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            tls_cert_path: Some("/tmp/cert.pem".into()),
            tls_key_path: None,
            ..Default::default()
        });
        let err =
            ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap_err();
        assert!(matches!(err, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_v2_sanitize_tls_key_without_cert() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            tls_cert_path: None,
            tls_key_path: Some("/tmp/key.pem".into()),
            ..Default::default()
        });
        let err =
            ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap_err();
        assert!(matches!(err, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_v2_sanitize_tls_both_paths_ok() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            tls_cert_path: Some("/tmp/cert.pem".into()),
            tls_key_path: Some("/tmp/key.pem".into()),
            ..Default::default()
        });
        ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap();
    }

    #[test]
    fn test_v2_sanitize_zero_websocket_connections() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            websocket_enabled: true,
            websocket_max_connections: 0,
            ..Default::default()
        });
        let err =
            ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap_err();
        assert!(matches!(err, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_v2_sanitize_zero_websocket_subs() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            websocket_enabled: true,
            websocket_max_subscriptions_per_conn: 0,
            ..Default::default()
        });
        let err =
            ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap_err();
        assert!(matches!(err, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_v2_sanitize_websocket_disabled_skips_ws_checks() {
        // When WebSocket is off, zero connections/subs are fine.
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            websocket_enabled: false,
            websocket_max_connections: 0,
            websocket_max_subscriptions_per_conn: 0,
            ..Default::default()
        });
        ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap();
    }

    #[test]
    fn test_v2_sanitize_zero_batch_size() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            json_rpc_batch_max_size: 0,
            ..Default::default()
        });
        let err =
            ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap_err();
        assert!(matches!(err, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_v2_sanitize_zero_content_length_limit() {
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            content_length_limit: Some(0),
            ..Default::default()
        });
        let err =
            ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap_err();
        assert!(matches!(err, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_v2_sanitize_content_length_limit_none_ok() {
        // None means "inherit from v1" -- always valid.
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            content_length_limit: None,
            ..Default::default()
        });
        ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap();
    }

    #[test]
    fn test_v2_sanitize_address_conflict_with_v1() {
        let v1_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let cfg = NodeConfig {
            api: ApiConfig {
                address: v1_addr,
                ..Default::default()
            },
            api_v2: ApiV2Config {
                enabled: true,
                address: Some(v1_addr), // same as v1!
                ..Default::default()
            },
            ..Default::default()
        };
        let err =
            ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap_err();
        assert!(matches!(err, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_v2_sanitize_separate_address_ok() {
        let cfg = NodeConfig {
            api: ApiConfig {
                address: "127.0.0.1:8080".parse().unwrap(),
                ..Default::default()
            },
            api_v2: ApiV2Config {
                enabled: true,
                address: Some("127.0.0.1:8081".parse().unwrap()),
                ..Default::default()
            },
            ..Default::default()
        };
        ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap();
    }

    #[test]
    fn test_v2_sanitize_cohost_no_address_ok() {
        // address = None means co-host on v1 port -- always valid.
        let cfg = v2_node_config(ApiV2Config {
            enabled: true,
            address: None,
            ..Default::default()
        });
        ApiV2Config::sanitize(&cfg, NodeType::Validator, None).unwrap();
    }
}
