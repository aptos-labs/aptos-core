// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replay a committed on-chain Aptos transaction locally, optionally with
//! local Move package module overrides.

use super::{
    super::{
        common::{mcp_err, mcp_invalid, tool_error},
        session::{into_call_tool_result, FlowSession},
    },
    replay_tracing::{CaptureOpts, TraceCapture, TraceRecorder, TracingDebugger},
};
use aptos_cli_common::format_txn_status;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_move_cli::{source_locator::AptosSourceLocator, MoveDebugger};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_types::{
    transaction::{
        ExecutionStatus, ReplayProtector, SignedTransaction, Transaction, TransactionOutput,
        TransactionStatus,
    },
    vm_status::AbortLocation,
};
use aptos_validator_interface::LocalModuleOverrides;
use move_core_types::{account_address::AccountAddress, vm_status::VMStatus};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};
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
    /// When true, capture a structured trace into the response. Adds
    /// exactly one `state_view { version, with_overrides }` entry per
    /// replay (whichever path the simulator took). Off by default; the
    /// wrapper costs one extra indirection on every storage read.
    #[serde(default)]
    trace: bool,
    /// When true, additionally record one `storage_read` entry per
    /// state-view read. Off by default because a single replay typically
    /// issues hundreds of reads, which crowd out the single `state_view`
    /// event. Only consulted when `trace` is true.
    #[serde(default)]
    trace_storage_reads: bool,
    /// Maximum number of trace entries before truncation. Defaults to 500.
    /// Only consulted when `trace` is true.
    #[serde(default = "default_max_trace_events")]
    max_trace_events: usize,
    /// When true, `storage_read` trace entries omit the `Debug`-formatted
    /// `StateKey`. Defaults to true. Only consulted when both `trace` and
    /// `trace_storage_reads` are true.
    #[serde(default = "default_redact")]
    redact_storage_keys: bool,
}

fn default_max_trace_events() -> usize {
    500
}

fn default_redact() -> bool {
    true
}

/// Server-side ceiling on `max_trace_events`. Generous enough for the
/// documented "raise it to a few thousand when capturing storage reads"
/// workflow, low enough that a malicious or buggy caller cannot drive
/// unbounded recorder growth. Each entry is at most a few hundred bytes,
/// so 100k entries bounds the recorder at ~tens of MB.
const MAX_TRACE_EVENTS_CAP: usize = 100_000;

/// Validate the user-supplied capture knobs. Only called when `trace` is
/// enabled; the other fields are unused otherwise and would needlessly
/// reject valid requests that left their defaults in place.
fn validate_capture_opts(max_trace_events: usize) -> Result<(), String> {
    if max_trace_events == 0 {
        return Err(
            "max_trace_events must be >= 1 when trace is enabled: a value of 0 would \
             drop the guaranteed `state_view` event"
                .to_string(),
        );
    }
    if max_trace_events > MAX_TRACE_EVENTS_CAP {
        return Err(format!(
            "max_trace_events ({}) exceeds the server-side cap of {}",
            max_trace_events, MAX_TRACE_EVENTS_CAP,
        ));
    }
    Ok(())
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
    /// Captured trace entries; only present when the request set `trace: true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<TraceCapture>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema, PartialEq, Eq)]
struct AbortDetails {
    /// `"0xADDR::module_name"` for module aborts, `"script"` for the script variant.
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
    /// Same format as [`AbortDetails::location`].
    location: String,
    /// Function index within the module.
    function: u16,
    /// Bytecode offset within the function.
    code_offset: u16,
}

fn format_abort_location(loc: &AbortLocation) -> String {
    match loc {
        AbortLocation::Module(m) => format!("{}::{}", m.address().to_hex_literal(), m.name()),
        AbortLocation::Script => "script".to_string(),
    }
}

fn abort_details_from(status: &ExecutionStatus) -> Option<AbortDetails> {
    match status {
        ExecutionStatus::MoveAbort {
            location,
            code,
            info,
        } => {
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
        },
        ExecutionStatus::Success
        | ExecutionStatus::OutOfGas
        | ExecutionStatus::ExecutionFailure { .. }
        | ExecutionStatus::MiscellaneousError(_) => None,
    }
}

fn execution_failure_details_from(status: &ExecutionStatus) -> Option<ExecutionFailureDetails> {
    match status {
        ExecutionStatus::ExecutionFailure {
            location,
            function,
            code_offset,
        } => Some(ExecutionFailureDetails {
            location: format_abort_location(location),
            function: *function,
            code_offset: *code_offset,
        }),
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

/// Construct an `AptosDebugger` against the given parsed REST endpoint.
/// Returns it as an `Arc<dyn MoveDebugger>` so the orchestrator can both
/// hand a clone to the [`TracingDebugger`] wrapper and call the inner
/// debugger directly for post-execution materialization.
fn build_debugger(
    base_url: AptosBaseUrl,
    node_api_key: Option<&str>,
) -> Result<Arc<dyn MoveDebugger>, rmcp::ErrorData> {
    let mut builder = Client::builder(base_url);
    if let Some(key) = node_api_key {
        builder = builder
            .api_key(key)
            .map_err(|e| mcp_invalid(format!("invalid node_api_key: {}", e)))?;
    }
    let debugger = AptosDebugger::rest_client(builder.build())
        .map_err(|e| mcp_err(format!("failed to construct debugger: {}", e)))?;
    Ok(Arc::new(debugger))
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
            if let legacy_move_compiler::compiled_unit::CompiledUnit::Module(ref named) = unit.unit
            {
                let module = &named.module;
                let mut bytes = vec![];
                module.serialize(&mut bytes).map_err(|e| {
                    format!("failed to serialize module {}: {}", module.self_id(), e)
                })?;
                let module_id = module.self_id();
                overrides.add_module(&module_id, bytes);

                let sm_bytes = unit.unit.serialize_source_map();
                let source_text = match std::fs::read_to_string(&unit.source_path) {
                    Ok(text) => text,
                    Err(e) => {
                        log::warn!(
                            "could not read source file {} for local override: {}",
                            unit.source_path.display(),
                            e
                        );
                        String::new()
                    },
                };
                let filename = unit.source_path.to_string_lossy().into_owned();
                if let Err(e) = locator.add_local_module(module, &sm_bytes, &source_text, &filename)
                {
                    log::warn!(
                        "could not load source map for module {}: {}",
                        module.self_id(),
                        e
                    );
                }
            }
        }
    }

    Ok((overrides, locator))
}

#[tool_router(router = replay_transaction_router, vis = "pub(crate)")]
impl FlowSession {
    /// Replay a committed on-chain transaction locally to debug its outcome.
    /// Supports optional local Move package overrides for testing patches.
    #[tool(
        description = "Replay a committed on-chain transaction locally to debug its outcome. Supports optional local Move package overrides.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn move_replay_transaction(
        &self,
        Parameters(params): Parameters<MoveReplayTransactionParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(self
            .move_replay_transaction_impl(params)
            .await
            .unwrap_or_else(tool_error))
    }
}

/// Construct the JSON response from the materialized transaction output and the
/// raw VM status. Mirrors the fields the CLI Replay command surfaces, plus the
/// structured abort/execution-failure details produced by our helpers.
fn build_response(
    txn: &SignedTransaction,
    version: u64,
    txn_output: TransactionOutput,
    vm_status: &VMStatus,
    local_override_in_use: bool,
    trace: Option<TraceCapture>,
) -> ReplayResponse {
    let status = txn_output.status();
    let exec_status_opt = match status {
        TransactionStatus::Keep(e) => Some(e),
        TransactionStatus::Discard(_) | TransactionStatus::Retry => None,
    };
    let abort = exec_status_opt.and_then(abort_details_from);
    let execution_failure = exec_status_opt.and_then(execution_failure_details_from);

    let sequence_number = match txn.replay_protector() {
        ReplayProtector::SequenceNumber(s) => Some(s),
        ReplayProtector::Nonce(_) => None,
    };

    ReplayResponse {
        success: success_from(status),
        vm_status: format_txn_status(status, vm_status),
        abort,
        execution_failure,
        transaction_hash: txn.committed_hash().to_string(),
        version,
        sender: txn.sender().to_hex_literal(),
        sequence_number,
        gas_used: txn_output.gas_used(),
        gas_unit_price: txn.gas_unit_price(),
        local_override_in_use,
        trace,
    }
}

impl FlowSession {
    async fn move_replay_transaction_impl(
        &self,
        params: MoveReplayTransactionParams,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!(
            "move_replay_transaction: txn_id={} network=`{}` local_packages={}",
            params.txn_id,
            params.network,
            params.local_package_paths.len()
        );

        // Validate user-supplied inputs up front. These are categorized as
        // `invalid_params` so the MCP client can distinguish them from
        // runtime failures further down.
        let base_url = parse_network(&params.network).map_err(mcp_invalid)?;
        let pkg_paths = validate_package_paths(&params.local_package_paths).map_err(mcp_invalid)?;
        let named = validate_named_addresses(&params.named_addresses).map_err(mcp_invalid)?;
        if params.trace {
            validate_capture_opts(params.max_trace_events).map_err(mcp_invalid)?;
        }

        // Build the debugger eagerly: we need it for both the async fetch
        // below and the blocking VM run inside `spawn_blocking`.
        let debugger = build_debugger(base_url, params.node_api_key.as_deref())?;

        // Fetch the transaction (async, network I/O).
        let txn_id = params.txn_id;
        let (txn, _txn_info, aux_info) = debugger
            .get_committed_transaction_at_version(txn_id)
            .await
            .map_err(|e| mcp_err(format!("failed to fetch transaction {}: {}", txn_id, e)))?;

        let user_txn = require_user_transaction(txn, txn_id)?;
        let hash = user_txn.committed_hash();

        let local_override_in_use = !pkg_paths.is_empty();
        let tool_timeout = self.tool_timeout();

        // Build a recorder iff tracing was requested. The wrapper around
        // the debugger holds a clone of this `Arc`; we drop the wrapper
        // before `into_capture` so `Arc::try_unwrap` is guaranteed to
        // succeed.
        let recorder = params.trace.then(|| {
            TraceRecorder::new(CaptureOpts {
                max_events: params.max_trace_events,
                record_storage_reads: params.trace_storage_reads,
                redact_storage_keys: params.redact_storage_keys,
            })
        });

        // Offload VM execution (and any local-package compilation) to a
        // blocking thread.
        let result = tokio::time::timeout(
            tool_timeout,
            tokio::task::spawn_blocking(move || -> Result<ReplayResponse, String> {
                // The wrapper, if any, holds an `Arc` clone of the inner
                // debugger. The bare `inner` Arc lets the materialization
                // step bypass the wrapper so it does not pollute the trace.
                let wrapper: Option<TracingDebugger> = recorder
                    .clone()
                    .map(|rec| TracingDebugger::new(Arc::clone(&debugger), rec));
                let vm_debugger: &dyn MoveDebugger = match &wrapper {
                    Some(w) => w,
                    None => debugger.as_ref(),
                };

                let (vm_status, vm_output) = if local_override_in_use {
                    let (overrides, locator) = build_local_overrides(&pkg_paths, &named)?;
                    let overrides = Arc::new(overrides);
                    let locator: Arc<dyn move_vm_runtime::source_locator::SourceLocator> =
                        Arc::new(locator);
                    aptos_move_cli::local_simulation::run_transaction_with_local_overrides(
                        vm_debugger,
                        txn_id,
                        user_txn.clone(),
                        aux_info,
                        overrides,
                        Some(locator),
                    )
                    .map_err(|e| format!("VM execution failed: {}", e))?
                } else {
                    aptos_move_cli::local_simulation::run_transaction_using_debugger(
                        vm_debugger,
                        txn_id,
                        user_txn.clone(),
                        hash,
                        aux_info,
                    )
                    .map_err(|e| format!("VM execution failed: {}", e))?
                };

                // Drop the wrapper so `into_capture` below has the sole
                // `Arc<TraceRecorder>` clone. Materialization uses the
                // unwrapped `debugger` directly, so dropping the wrapper
                // first does not affect it.
                drop(wrapper);

                // Materialize the VMOutput so we can read gas + status.
                let materialize_result = vm_output.into_transaction_output();

                // Drain the trace after materialization. Doing it here lets
                // us either embed it in the success response or append it to
                // the error message — losing trace data on materialization
                // failure would leave the caller blind to the reads that
                // preceded it.
                let trace = recorder.map(TraceRecorder::into_capture);

                let txn_output = match materialize_result {
                    Ok(o) => o,
                    Err(e) => {
                        let trace_suffix = trace
                            .as_ref()
                            .and_then(|t| serde_json::to_string(t).ok())
                            .map(|s| format!(" — captured trace: {}", s))
                            .unwrap_or_default();
                        return Err(format!(
                            "failed to materialize transaction output: {}{}",
                            e, trace_suffix,
                        ));
                    },
                };

                Ok(build_response(
                    &user_txn,
                    txn_id,
                    txn_output,
                    &vm_status,
                    local_override_in_use,
                    trace,
                ))
            }),
        )
        .await
        .map_err(|_| {
            mcp_err(format!(
                "tool timeout ({}s exceeded)",
                tool_timeout.as_secs()
            ))
        })?;

        let response = result
            .map_err(|e| mcp_err(format!("replay task error: {}", e)))?
            .map_err(mcp_err)?;

        Ok(into_call_tool_result(&response))
    }
}

/// Unwrap a fetched `Transaction` into the `UserTransaction` variant, or
/// surface a structured `invalid_params` error naming the rejected variant.
fn require_user_transaction(
    txn: Transaction,
    txn_id: u64,
) -> Result<SignedTransaction, rmcp::ErrorData> {
    match txn {
        Transaction::UserTransaction(t) => Ok(t),
        other => {
            let variant = match other {
                Transaction::GenesisTransaction(_) => "Genesis",
                Transaction::BlockMetadata(_) => "BlockMetadata",
                Transaction::BlockMetadataExt(_) => "BlockMetadataExt",
                Transaction::BlockEpilogue(_) => "BlockEpilogue",
                Transaction::StateCheckpoint(_) => "StateCheckpoint",
                Transaction::ValidatorTransaction(_) => "ValidatorTransaction",
                Transaction::UserTransaction(_) => unreachable!(),
            };
            Err(mcp_invalid(format!(
                "transaction at version {} is a {} transaction; only user transactions are supported. \
                 Use `aptos move replay` directly for system transactions.",
                txn_id, variant,
            )))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_rest_client::AptosBaseUrl;
    use aptos_types::vm_status::AbortLocation;
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    };

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
        assert!(matches!(
            parse_network("mainnet"),
            Ok(AptosBaseUrl::Mainnet)
        ));
        assert!(matches!(
            parse_network("testnet"),
            Ok(AptosBaseUrl::Testnet)
        ));
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
        assert_eq!(
            details.description.as_deref(),
            Some("Not enough balance to withdraw")
        );
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
    fn tool_is_registered() {
        let names = FlowSession::tool_names();
        assert!(
            names.iter().any(|n| n == "move_replay_transaction"),
            "expected move_replay_transaction in {:?}",
            names
        );
    }

    #[test]
    fn validate_capture_opts_rejects_zero() {
        let err = validate_capture_opts(0).unwrap_err();
        assert!(
            err.contains("max_trace_events must be >= 1"),
            "got: {}",
            err
        );
    }

    #[test]
    fn validate_capture_opts_rejects_above_cap() {
        let err = validate_capture_opts(MAX_TRACE_EVENTS_CAP + 1).unwrap_err();
        assert!(err.contains("exceeds the server-side cap"), "got: {}", err);
    }

    #[test]
    fn validate_capture_opts_accepts_one_and_cap_and_default() {
        validate_capture_opts(1).expect("1 is the minimum allowed");
        validate_capture_opts(default_max_trace_events()).expect("default must be valid");
        validate_capture_opts(MAX_TRACE_EVENTS_CAP).expect("cap is inclusive");
    }

    #[test]
    fn require_user_transaction_accepts_user_variant() {
        let raw = aptos_types::transaction::RawTransaction::new(
            AccountAddress::ONE,
            0,
            aptos_types::transaction::TransactionPayload::Script(
                aptos_types::transaction::Script::new(vec![], vec![], vec![]),
            ),
            0,
            0,
            0,
            aptos_types::chain_id::ChainId::test(),
        );
        let signed = SignedTransaction::new(
            raw,
            aptos_crypto::ed25519::Ed25519PublicKey::try_from(&[0u8; 32][..]).unwrap(),
            aptos_crypto::ed25519::Ed25519Signature::try_from(&[0u8; 64][..]).unwrap(),
        );
        let txn = Transaction::UserTransaction(signed.clone());
        let got = require_user_transaction(txn, 42).expect("user txn should pass through");
        assert_eq!(got.sender(), signed.sender());
    }

    #[test]
    fn require_user_transaction_rejects_state_checkpoint() {
        let txn = Transaction::StateCheckpoint(aptos_crypto::HashValue::zero());
        let err = require_user_transaction(txn, 7).unwrap_err();
        assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
        assert!(
            err.message.contains("StateCheckpoint"),
            "expected variant name in message: {}",
            err.message
        );
        assert!(
            err.message.contains("version 7"),
            "expected version in message: {}",
            err.message
        );
    }

    #[test]
    fn require_user_transaction_rejects_genesis() {
        // GenesisTransaction needs a WriteSetPayload; use the empty `Direct` form.
        let txn =
            Transaction::GenesisTransaction(aptos_types::transaction::WriteSetPayload::Direct(
                aptos_types::transaction::ChangeSet::new(
                    aptos_types::write_set::WriteSet::default(),
                    vec![],
                ),
            ));
        let err = require_user_transaction(txn, 0).unwrap_err();
        assert_eq!(err.code, rmcp::model::ErrorCode::INVALID_PARAMS);
        assert!(
            err.message.contains("Genesis"),
            "expected variant name in message: {}",
            err.message
        );
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
        assert!(
            !overrides.is_empty(),
            "overrides should contain at least one module"
        );
    }
}
