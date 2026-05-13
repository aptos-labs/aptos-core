// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replay a committed on-chain Aptos transaction locally, optionally with
//! local Move package module overrides.

use super::super::session::FlowSession;
use rmcp::{handler::server::router::tool::ToolRouter, tool_router};

use aptos_rest_client::AptosBaseUrl;
use rmcp::schemars;
use std::collections::BTreeMap;
use url::Url;

/// Parse the `network` parameter into a base URL. Accepts the well-known names
/// `mainnet` / `testnet` / `devnet`, otherwise treats the input as a REST endpoint URL.
fn parse_network(s: &str) -> Result<AptosBaseUrl, String> {
    if s.is_empty() {
        return Err("network must not be empty".to_string());
    }
    match s {
        "mainnet" => Ok(AptosBaseUrl::Mainnet),
        "testnet" => Ok(AptosBaseUrl::Testnet),
        "devnet" => Ok(AptosBaseUrl::Devnet),
        other => Url::parse(other)
            .map(AptosBaseUrl::Custom)
            .map_err(|e| format!("invalid network `{}`: {}. Use 'mainnet', 'testnet', 'devnet', or a REST endpoint URL.", other, e)),
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MoveReplayTransactionParams {
    /// The committed transaction version (ledger version) to replay.
    txn_id: u64,
    /// Network: "mainnet" | "testnet" | "devnet" | a REST endpoint URL.
    network: String,
    /// Optional: paths to local Move packages whose modules override
    /// the on-chain versions during replay. Each path must contain Move.toml.
    #[serde(default)]
    local_package_paths: Vec<String>,
    /// Optional: named-address bindings for compiling local packages.
    /// Maps "name" → "0xADDR". Only used when local_package_paths is non-empty.
    #[serde(default)]
    named_addresses: BTreeMap<String, String>,
    /// Optional: API key sent as `Authorization: Bearer <key>`.
    #[serde(default)]
    node_api_key: Option<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct ReplayResponse {
    /// true = Keep(Success), false = Keep(any failure), null = Discard/Retry.
    success: Option<bool>,
    /// Formatted vm-status string (same as the CLI shows via format_txn_status).
    vm_status: String,
    /// Structured info when status is MoveAbort.
    #[serde(skip_serializing_if = "Option::is_none")]
    abort: Option<AbortDetails>,
    /// Structured info when status is ExecutionFailure.
    #[serde(skip_serializing_if = "Option::is_none")]
    execution_failure: Option<ExecutionFailureDetails>,
    transaction_hash: String,
    version: u64,
    sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence_number: Option<u64>,
    gas_used: u64,
    gas_unit_price: u64,
    /// True when local_package_paths was non-empty (replay diverged from on-chain).
    local_override_in_use: bool,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema, PartialEq, Eq)]
struct AbortDetails {
    /// "0xADDR::module_name" or "script".
    location: String,
    /// Raw abort code from the Move source.
    code: u64,
    /// Symbolic reason name (e.g. "EINSUFFICIENT_BALANCE") if present in module metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    /// Human-readable description from module metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema, PartialEq, Eq)]
struct ExecutionFailureDetails {
    /// "0xADDR::module_name" or "script".
    location: String,
    /// Function index within the module.
    function: u16,
    /// Bytecode offset within the function.
    code_offset: u16,
}

#[tool_router(router = replay_transaction_router, vis = "pub(crate)")]
impl FlowSession {}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_rest_client::AptosBaseUrl;

    #[test]
    fn parse_network_known_names() {
        assert!(matches!(parse_network("mainnet"), Ok(AptosBaseUrl::Mainnet)));
        assert!(matches!(parse_network("testnet"), Ok(AptosBaseUrl::Testnet)));
        assert!(matches!(parse_network("devnet"), Ok(AptosBaseUrl::Devnet)));
    }

    #[test]
    fn parse_network_custom_url() {
        let url = "https://my-node.example.com/v1";
        let parsed = parse_network(url).expect("valid url should parse");
        match parsed {
            AptosBaseUrl::Custom(u) => assert_eq!(u.as_str(), "https://my-node.example.com/v1"),
            _ => panic!("expected Custom(...), got a non-Custom AptosBaseUrl variant"),
        }
    }

    #[test]
    fn parse_network_rejects_empty() {
        assert!(parse_network("").is_err());
    }

    #[test]
    fn parse_network_rejects_garbage() {
        assert!(parse_network("not a url").is_err());
    }
}
