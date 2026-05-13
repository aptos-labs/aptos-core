// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replay a committed on-chain Aptos transaction locally, optionally with
//! local Move package module overrides.

use super::super::session::FlowSession;
use rmcp::{handler::server::router::tool::ToolRouter, tool_router};

use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_move_cli::source_locator::AptosSourceLocator;
use aptos_move_cli::MoveDebugger;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use aptos_types::vm_status::AbortLocation;
use aptos_validator_interface::LocalModuleOverrides;
use move_core_types::account_address::AccountAddress;
use rmcp::schemars;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
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

/// Format an `AbortLocation` as `"0xADDR::module_name"` (for `Module` variants) or
/// `"script"` (for the `Script` variant).
fn format_abort_location(loc: &AbortLocation) -> String {
    match loc {
        AbortLocation::Module(m) => format!("{}::{}", m.address().to_hex_literal(), m.name()),
        AbortLocation::Script => "script".to_string(),
    }
}

fn abort_details_from(status: &ExecutionStatus) -> Option<AbortDetails> {
    match status {
        ExecutionStatus::MoveAbort { location, code, info } => {
            let (reason, description) = match info {
                Some(i) => (
                    (!i.reason_name.is_empty()).then(|| i.reason_name.clone()),
                    (!i.description.is_empty()).then(|| i.description.clone()),
                ),
                None => (None, None),
            };
            Some(AbortDetails {
                location: format_abort_location(location),
                code: *code,
                reason,
                description,
            })
        }
        ExecutionStatus::Success
        | ExecutionStatus::OutOfGas
        | ExecutionStatus::ExecutionFailure { .. }
        | ExecutionStatus::MiscellaneousError(_) => None,
    }
}

fn execution_failure_details_from(status: &ExecutionStatus) -> Option<ExecutionFailureDetails> {
    match status {
        ExecutionStatus::ExecutionFailure { location, function, code_offset } => {
            Some(ExecutionFailureDetails {
                location: format_abort_location(location),
                function: *function,
                code_offset: *code_offset,
            })
        }
        ExecutionStatus::Success
        | ExecutionStatus::OutOfGas
        | ExecutionStatus::MoveAbort { .. }
        | ExecutionStatus::MiscellaneousError(_) => None,
    }
}

fn success_from(status: &TransactionStatus) -> Option<bool> {
    match status {
        TransactionStatus::Keep(exec) => Some(exec.is_success()),
        TransactionStatus::Discard(_) | TransactionStatus::Retry => None,
    }
}

fn validate_package_paths(paths: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        let path = Path::new(p);
        if !path.exists() {
            return Err(format!("local package path `{}` does not exist", p));
        }
        if !path.is_dir() {
            return Err(format!("local package path `{}` is not a directory", p));
        }
        if !path.join("Move.toml").exists() {
            return Err(format!(
                "local package path `{}` is not a Move package (no Move.toml)",
                p
            ));
        }
        out.push(path.to_path_buf());
    }
    Ok(out)
}

fn validate_named_addresses(
    addrs: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, AccountAddress>, String> {
    addrs
        .iter()
        .map(|(k, v)| {
            AccountAddress::from_str(v)
                .map(|a| (k.clone(), a))
                .map_err(|e| format!("invalid named address `{}={}`: {}", k, v, e))
        })
        .collect()
}

fn build_debugger(
    network: &str,
    node_api_key: Option<&str>,
) -> Result<Box<dyn MoveDebugger>, String> {
    let base_url = parse_network(network)?;
    let mut builder = Client::builder(base_url);
    if let Some(key) = node_api_key {
        builder = builder
            .api_key(key)
            .map_err(|e| format!("invalid node_api_key: {}", e))?;
    }
    let client = builder.build();
    let debugger = AptosDebugger::rest_client(client)
        .map_err(|e| format!("failed to construct debugger: {}", e))?;
    Ok(Box::new(debugger))
}

/// Compile each local Move package and collect the resulting modules into a
/// [`LocalModuleOverrides`] map plus an [`AptosSourceLocator`] populated with
/// the source maps. Used to swap on-chain modules for locally-compiled
/// versions during replay.
fn build_local_overrides(
    package_paths: &[PathBuf],
    named_addresses: &BTreeMap<String, AccountAddress>,
) -> Result<(LocalModuleOverrides, AptosSourceLocator), String> {
    let mut overrides = LocalModuleOverrides::new();
    let mut locator = AptosSourceLocator::new();

    for pkg_path in package_paths {
        let built = BuiltPackage::build(pkg_path.clone(), BuildOptions {
            with_srcs: true,
            with_source_maps: true,
            forced_named_addresses: named_addresses.clone(),
            ..BuildOptions::default()
        })
        .map_err(|e| {
            format!(
                "failed to build local package at {}: {}",
                pkg_path.display(),
                e
            )
        })?;

        for unit in built.package.root_modules() {
            if let legacy_move_compiler::compiled_unit::CompiledUnit::Module(ref named) =
                unit.unit
            {
                let module = &named.module;
                let mut bytes = vec![];
                module.serialize(&mut bytes).map_err(|e| {
                    format!("failed to serialize module {}: {}", module.self_id(), e)
                })?;
                let module_id = module.self_id();
                overrides.add_module(&module_id, bytes);

                let sm_bytes = unit.unit.serialize_source_map();
                let source_text =
                    std::fs::read_to_string(&unit.source_path).unwrap_or_default();
                let filename = unit.source_path.to_string_lossy().into_owned();
                let _ = locator.add_local_module(module, &sm_bytes, &source_text, &filename);
            }
        }
    }

    Ok((overrides, locator))
}

#[tool_router(router = replay_transaction_router, vis = "pub(crate)")]
impl FlowSession {}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_rest_client::AptosBaseUrl;
    use aptos_types::vm_status::AbortLocation;
    use move_core_types::account_address::AccountAddress;
    use move_core_types::identifier::Identifier;
    use move_core_types::language_storage::ModuleId;

    #[test]
    fn abort_location_module() {
        let module_id = ModuleId::new(AccountAddress::ONE, Identifier::new("coin").unwrap());
        let loc = AbortLocation::Module(module_id);
        assert_eq!(format_abort_location(&loc), "0x1::coin");
    }

    #[test]
    fn abort_location_script() {
        assert_eq!(format_abort_location(&AbortLocation::Script), "script");
    }

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

    #[test]
    fn abort_details_with_info() {
        let info = aptos_types::transaction::AbortInfo {
            reason_name: "EINSUFFICIENT_BALANCE".to_string(),
            description: "Not enough balance to withdraw".to_string(),
        };
        let status = aptos_types::transaction::ExecutionStatus::MoveAbort {
            location: AbortLocation::Module(ModuleId::new(
                AccountAddress::ONE,
                Identifier::new("coin").unwrap(),
            )),
            code: 65540,
            info: Some(info),
        };
        let details = abort_details_from(&status).expect("expected Some for MoveAbort");
        assert_eq!(details.location, "0x1::coin");
        assert_eq!(details.code, 65540);
        assert_eq!(details.reason.as_deref(), Some("EINSUFFICIENT_BALANCE"));
        assert_eq!(details.description.as_deref(), Some("Not enough balance to withdraw"));
    }

    #[test]
    fn abort_details_without_info() {
        let status = aptos_types::transaction::ExecutionStatus::MoveAbort {
            location: AbortLocation::Script,
            code: 13,
            info: None,
        };
        let details = abort_details_from(&status).expect("expected Some for MoveAbort");
        assert_eq!(details.location, "script");
        assert_eq!(details.code, 13);
        assert!(details.reason.is_none());
        assert!(details.description.is_none());
    }

    #[test]
    fn abort_details_with_empty_info_drops_fields() {
        let info = aptos_types::transaction::AbortInfo {
            reason_name: String::new(),
            description: String::new(),
        };
        let status = aptos_types::transaction::ExecutionStatus::MoveAbort {
            location: AbortLocation::Script,
            code: 7,
            info: Some(info),
        };
        let details = abort_details_from(&status).expect("expected Some for MoveAbort");
        assert!(details.reason.is_none());
        assert!(details.description.is_none());
    }

    #[test]
    fn abort_details_none_for_non_abort() {
        let status = aptos_types::transaction::ExecutionStatus::Success;
        assert!(abort_details_from(&status).is_none());

        let status = aptos_types::transaction::ExecutionStatus::OutOfGas;
        assert!(abort_details_from(&status).is_none());
    }

    #[test]
    fn execution_failure_details_module() {
        let status = ExecutionStatus::ExecutionFailure {
            location: AbortLocation::Module(ModuleId::new(
                AccountAddress::ONE,
                Identifier::new("vector").unwrap(),
            )),
            function: 3,
            code_offset: 42,
        };
        let details = execution_failure_details_from(&status).expect("expected Some");
        assert_eq!(details.location, "0x1::vector");
        assert_eq!(details.function, 3);
        assert_eq!(details.code_offset, 42);
    }

    #[test]
    fn execution_failure_details_none_for_other_status() {
        let status = ExecutionStatus::Success;
        assert!(execution_failure_details_from(&status).is_none());
    }

    #[test]
    fn success_from_keep_success() {
        let status = aptos_types::transaction::TransactionStatus::Keep(ExecutionStatus::Success);
        assert_eq!(success_from(&status), Some(true));
    }

    #[test]
    fn success_from_keep_failure_variants() {
        let oog = aptos_types::transaction::TransactionStatus::Keep(ExecutionStatus::OutOfGas);
        assert_eq!(success_from(&oog), Some(false));

        let abort = aptos_types::transaction::TransactionStatus::Keep(ExecutionStatus::MoveAbort {
            location: AbortLocation::Script,
            code: 0,
            info: None,
        });
        assert_eq!(success_from(&abort), Some(false));
    }

    #[test]
    fn success_from_discard_or_retry() {
        let discard = aptos_types::transaction::TransactionStatus::Discard(
            aptos_types::vm_status::StatusCode::INVALID_SIGNATURE,
        );
        assert_eq!(success_from(&discard), None);

        let retry = aptos_types::transaction::TransactionStatus::Retry;
        assert_eq!(success_from(&retry), None);
    }

    #[test]
    fn validate_package_paths_accepts_real_package() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Move.toml"), "[package]\nname = \"x\"\n").unwrap();
        let paths = vec![tmp.path().to_string_lossy().into_owned()];
        let resolved = validate_package_paths(&paths).expect("should accept valid package");
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0], tmp.path());
    }

    #[test]
    fn validate_package_paths_rejects_missing_dir() {
        let paths = vec!["/no/such/path/12345xyz".to_string()];
        let err = validate_package_paths(&paths).unwrap_err();
        assert!(err.contains("does not exist"), "got: {}", err);
    }

    #[test]
    fn validate_package_paths_rejects_missing_manifest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let paths = vec![tmp.path().to_string_lossy().into_owned()];
        let err = validate_package_paths(&paths).unwrap_err();
        assert!(err.contains("Move.toml"), "got: {}", err);
    }

    #[test]
    fn validate_named_addresses_accepts_hex_addr() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("my_module".to_string(), "0x1".to_string());
        let parsed = validate_named_addresses(&m).expect("should accept hex");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.get("my_module").unwrap(), &AccountAddress::ONE);
    }

    #[test]
    fn validate_named_addresses_rejects_garbage() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("bad".to_string(), "not an address".to_string());
        let err = validate_named_addresses(&m).unwrap_err();
        assert!(err.contains("bad"), "got: {}", err);
    }

    #[test]
    fn build_local_overrides_produces_module() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let pkg = std::path::PathBuf::from(manifest_dir)
            .join("src/tests/move_replay_transaction/fixtures/empty_pkg");
        let paths = vec![pkg];
        let named = std::collections::BTreeMap::new();
        let result = build_local_overrides(&paths, &named);
        assert!(result.is_ok(), "should build overrides: {:?}", result.err());
        let (overrides, _locator) = result.unwrap();
        assert!(!overrides.is_empty(), "overrides should contain at least one module");
    }
}
