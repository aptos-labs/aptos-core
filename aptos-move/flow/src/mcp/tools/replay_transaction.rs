// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replay a committed on-chain Aptos transaction locally, optionally with
//! local Move package module overrides.

use super::super::session::FlowSession;
use rmcp::{handler::server::router::tool::ToolRouter, tool_router};

use aptos_rest_client::AptosBaseUrl;
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
