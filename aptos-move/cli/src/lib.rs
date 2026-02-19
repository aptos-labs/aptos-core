// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Aptos Move CLI - standalone Move tooling commands.
//!
//! This crate provides all `aptos move` subcommands. It can be used either:
//! - As part of the full Aptos CLI (which registers an `AptosContext` for network commands)
//! - As a standalone `move` binary (local-only commands work; network commands give a clear error)

pub mod aptos_debug_natives;
mod bytecode;
mod commands;
pub mod coverage;
mod fmt;
mod lint;
pub mod local_simulation;
mod manifest;
pub mod move_types;
pub mod package_hooks;
mod resource_account;
mod script_compile;
mod show;
mod sim;
pub mod stored_package;
mod tool_paths;
mod transactions;

#[cfg(test)]
mod tests;

// ── AptosContext trait for network operations ──

use aptos_cli_common::{CliError, CliTypedResult, TransactionOptions, TransactionSummary};
use aptos_gas_profiling::TransactionGasLog;
use aptos_rest_client::Client;
use aptos_types::{
    state_store::{
        errors::StateViewError, state_key::StateKey, state_slot::StateSlot,
        state_storage_usage::StateStorageUsage, state_value::StateValue, StateViewId, TStateView,
    },
    transaction::{
        AuxiliaryInfo, PersistedAuxiliaryInfo, SignedTransaction, Transaction, TransactionInfo,
        TransactionPayload, Version,
    },
};
use aptos_vm_types::output::VMOutput;
use async_trait::async_trait;
pub use commands::*;
use move_core_types::vm_status::VMStatus;
pub use move_types::{
    ArgWithTypeVec, ChunkedPublishOption, EntryFunctionArguments, LargePackagesModuleOption,
    MovePackageOptions, OverrideSizeCheckOption, ScriptFunctionArguments, TypeArgVec,
};
pub use package_hooks::register_package_hooks;
pub use resource_account::{ResourceAccountSeed, SeedEncoding};
pub use script_compile::{compile_in_temp_dir, CompileScriptFunction};
use std::sync::Arc;
pub use stored_package::CachedPackageMetadata;

/// Trait for command structs that have an `env: Arc<MoveEnv>` field.
///
/// This allows setting the environment after construction (e.g., after `Parser::parse_from`
/// where `#[clap(skip)]` defaults `env` to an empty `MoveEnv`).
pub trait WithMoveEnv: Sized {
    fn attach_env(self, env: Arc<MoveEnv>) -> Self;
}

/// Trait for network-dependent operations that require the full Aptos CLI environment.
///
/// The full Aptos CLI provides a real implementation via [`MoveEnv`]; the standalone
/// Move CLI uses a default `MoveEnv` without an `AptosContext`, so network commands
/// give a clear error directing users to the full CLI.
#[async_trait]
pub trait AptosContext: Send + Sync + 'static {
    /// Submit a transaction to the blockchain.
    async fn submit_transaction(
        &self,
        options: &TransactionOptions,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary>;

    /// Execute a view function.
    async fn view(
        &self,
        options: &TransactionOptions,
        request: aptos_rest_client::aptos_api_types::ViewRequest,
    ) -> CliTypedResult<Vec<serde_json::Value>>;
}

/// Environment providing external components (network context, debugger) to Move CLI commands.
///
/// The full Aptos CLI constructs this with real implementations. The standalone `move` binary
/// uses `MoveEnv::default()` (no network components), so network commands give a clear error.
#[derive(Default)]
pub struct MoveEnv {
    aptos_context: Option<Box<dyn AptosContext>>,
    debugger_factory: Option<MoveDebuggerFactory>,
}

impl std::fmt::Debug for MoveEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoveEnv")
            .field("aptos_context", &self.aptos_context.is_some())
            .field("debugger_factory", &self.debugger_factory.is_some())
            .finish()
    }
}

impl MoveEnv {
    pub fn new(
        aptos_context: Box<dyn AptosContext>,
        debugger_factory: MoveDebuggerFactory,
    ) -> Self {
        Self {
            aptos_context: Some(aptos_context),
            debugger_factory: Some(debugger_factory),
        }
    }

    /// Get the AptosContext, or an error if not available (standalone mode).
    pub fn aptos_context(&self) -> CliTypedResult<&dyn AptosContext> {
        self.aptos_context.as_deref().ok_or_else(|| {
            CliError::UnexpectedError(
                "This command requires the full Aptos CLI environment. \
                 Transaction submission and chain interaction are not available \
                 in standalone mode. Use the full `aptos` CLI instead."
                    .into(),
            )
        })
    }

    /// Create a MoveDebugger using the registered factory, or an error if not available.
    pub fn create_move_debugger(&self, client: Client) -> CliTypedResult<Box<dyn MoveDebugger>> {
        let factory = self.debugger_factory.as_ref().ok_or_else(|| {
            CliError::UnexpectedError(
                "Debugger not available in standalone mode. Use the full `aptos` CLI.".into(),
            )
        })?;
        factory(client).map_err(|e| CliError::UnexpectedError(e.to_string()))
    }
}

// ── DynStateView: type-erased wrapper for StateView ──

/// A type-erased wrapper around a `StateView` implementation.
///
/// This is used by the [`MoveDebugger`] trait to return state views without
/// exposing the concrete `DebuggerStateView` type from `aptos-validator-interface`.
pub struct DynStateView(Box<dyn TStateView<Key = StateKey> + Send + Sync>);

impl DynStateView {
    pub fn new(inner: Box<dyn TStateView<Key = StateKey> + Send + Sync>) -> Self {
        Self(inner)
    }
}

impl TStateView for DynStateView {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.0.id()
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        self.0.get_usage()
    }

    fn next_version(&self) -> Version {
        self.0.next_version()
    }

    fn get_state_slot(&self, state_key: &StateKey) -> Result<StateSlot, StateViewError> {
        self.0.get_state_slot(state_key)
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>, StateViewError> {
        self.0.get_state_value(state_key)
    }
}

// ── MoveDebugger trait for decoupling from aptos-move-debugger ──

/// Trait abstracting the debugger for local transaction execution.
///
/// The full Aptos CLI registers the real `AptosDebugger` implementation at startup.
/// The standalone Move CLI does not, so commands needing the debugger give a clear error.
#[async_trait]
pub trait MoveDebugger: Send + Sync + 'static {
    /// Get a state view at a specific chain version (for local VM execution).
    fn state_view_at_version(&self, version: u64) -> DynStateView;

    /// Execute a transaction with gas profiling enabled.
    fn execute_transaction_at_version_with_gas_profiler(
        &self,
        version: u64,
        txn: SignedTransaction,
        auxiliary_info: AuxiliaryInfo,
    ) -> anyhow::Result<(VMStatus, VMOutput, TransactionGasLog)>;

    /// Fetch a committed transaction at a specific version from the chain.
    async fn get_committed_transaction_at_version(
        &self,
        version: u64,
    ) -> anyhow::Result<(Transaction, TransactionInfo, PersistedAuxiliaryInfo)>;
}

type MoveDebuggerFactory =
    Box<dyn Fn(Client) -> anyhow::Result<Box<dyn MoveDebugger>> + Send + Sync>;

/// Submit a transaction via the registered [`AptosContext`].
///
/// Returns an error if no `AptosContext` is registered (standalone mode).
pub async fn dispatch_transaction(
    payload: TransactionPayload,
    txn_options_ref: &TransactionOptions,
    env: &MoveEnv,
) -> CliTypedResult<TransactionSummary> {
    let ctx = env.aptos_context()?;
    ctx.submit_transaction(txn_options_ref, payload).await
}
