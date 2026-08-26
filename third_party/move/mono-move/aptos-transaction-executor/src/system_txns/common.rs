// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Common utilities shared by all system transactions.

use crate::{
    calls::{call_system_function_unmetered, into_result, invariant_violation},
    errors::{ExecutionStatus, MoveExecutionFailure},
    executor::AptosTransactionExecutor,
    natives::extensions_with,
    outcome::TxnOutcome,
};
use anyhow::anyhow;
use aptos_types::{
    block_metadata::BlockMetadata,
    block_metadata_ext::BlockMetadataExt,
    fee_statement::FeeStatement,
    transaction::{BlockEpiloguePayload, SessionId},
};
use mono_move_core::{GasMeter, Interner};
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_natives::TransactionContextExtension;
use mono_move_runtime::InterpreterContext;
use move_core_types::{account_address::AccountAddress, ident_str, identifier::IdentStr};
use serde::Serialize;

const BLOCK: &IdentStr = ident_str!("block");

/// A system transaction's session identity: what seeds its transaction
/// context in place of user-transaction metadata.
pub(super) struct SystemTxnMetadata {
    pub txn_hash: [u8; 32],
    pub session_counter: u8,
}

impl SystemTxnMetadata {
    /// The session identity of a block-metadata transaction.
    pub fn for_block_metadata(block_metadata: &BlockMetadata) -> Self {
        Self::from_session_id(SessionId::block_meta(block_metadata))
    }

    /// The session identity of an extended block-metadata transaction.
    pub fn for_block_metadata_ext(block_metadata_ext: &BlockMetadataExt) -> Self {
        Self::from_session_id(SessionId::block_meta_ext(block_metadata_ext))
    }

    /// The session identity of a block-epilogue transaction.
    pub fn for_block_epilogue(block_epilogue: &BlockEpiloguePayload) -> Self {
        let block_id = match block_epilogue {
            BlockEpiloguePayload::V0 { block_id, .. }
            | BlockEpiloguePayload::V1 { block_id, .. }
            | BlockEpiloguePayload::V2 { block_id, .. } => *block_id,
        };
        Self::from_session_id(SessionId::block_epilogue(block_id))
    }

    fn from_session_id(session_id: SessionId) -> Self {
        Self {
            txn_hash: session_id.txn_hash(),
            session_counter: session_id.session_counter(),
        }
    }
}

impl<'a> AptosTransactionExecutor<'a> {
    /// A session for one system transaction, which runs everything unmetered.
    pub(super) fn system_session(&self, txn_data: &SystemTxnMetadata) -> InterpreterContext<'a> {
        // TODO(cleanup): make the loading policy configurable.
        let loader = Loader::new_with_policy(
            self.guard,
            self.module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            self.natives,
        );
        // A system transaction has no user transaction context and no script,
        // like the legacy VM's system sessions.
        let txn_context = TransactionContextExtension::new(
            txn_data.txn_hash,
            vec![],
            self.env.chain_id().id(),
            txn_data.session_counter,
            None,
        );
        // TODO(perf): give system sessions a smaller heap; block metadata
        // does not need the default size.
        //
        // Note: the zero-budget gas meter here acts as a tripwire: system code
        // is supposed to run through `call_system_function_unmetered`, which
        // grants free execution, and in case it is not and anything accidentally
        // gets metered, it fails on the first charge.
        InterpreterContext::new_idle(loader, GasMeter::new(0), self.data_provider, self.natives)
            .with_extensions(extensions_with(txn_context, self.usage))
    }
}

/// The outcome of a completed system transaction: fee-free and successful.
pub(super) fn system_txn_outcome<'guard>(interp: InterpreterContext<'guard>) -> TxnOutcome<'guard> {
    TxnOutcome::Executed {
        status: ExecutionStatus::Success,
        fee_statement: FeeStatement::zero(),
        effects: interp.finish(),
    }
}

/// BCS-encodes one system-call argument.
pub(super) fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, MoveExecutionFailure> {
    bcs::to_bytes(value)
        .map_err(|e| invariant_violation(anyhow!("system call argument does not serialize: {e}")))
        .map_err(MoveExecutionFailure::RuntimeError)
}

/// Calls `0x1::block::<function>` as the VM (`0x0`).
pub(super) fn call_block_function(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    function: &IdentStr,
    args: &[Vec<u8>],
) -> Result<(), MoveExecutionFailure> {
    let status = call_system_function_unmetered(
        guard,
        interp,
        &AccountAddress::ONE,
        BLOCK,
        function,
        guard.type_list_of(&[]),
        &[AccountAddress::ZERO.into_bytes()],
        args,
    )
    .map_err(MoveExecutionFailure::RuntimeError)?;
    into_result(status)
}
