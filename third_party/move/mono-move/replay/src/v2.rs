// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V2 harness: replays the whole transaction (prologue, payload, epilogue) on
//! the MonoMove-backed Aptos transaction executor.
//!
//! Gas is free (the executor's zero-gas mode), so the output carries no fee
//! effects and is byte-comparable with V1's. [`execute`] runs execution plus
//! materialization into a [`TransactionOutput`].

use anyhow::{anyhow, Result};
use aptos_types::{
    state_store::{state_key::StateKey, StateView},
    transaction::{
        AuxiliaryInfo, Transaction, TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
    },
};
use aptos_vm_environment::environment::AptosEnvironment;
use mono_move_aptos_state_view_providers::{StateViewModuleProvider, StateViewResourceProvider};
use mono_move_aptos_transaction_executor::{
    production_natives, AptosTransactionExecutor, DiscardReason, ExecutionStatus,
    MoveExecutionFailure, NoEffectsReason, TxnOutcome,
};
use mono_move_core::VMInternalError;
use mono_move_global_context::GlobalContext;
use mono_move_loader::LoaderError;
use mono_move_output::gap::{gap, Gap};
use move_binary_format::{access::ModuleAccess, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};

/// Something V1 runs that MonoMove does not support yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// The VM reported a gap.
    Vm(Gap),
    /// The executor rejected the transaction's shape.
    Transaction(&'static str),
    /// Writing the output hit a gap, after an execution that committed with `status`.
    Output { gap: Gap, status: TransactionStatus },
}

/// One execution on V2.
pub struct V2Run {
    /// The materialized output; `Err` only when materialization failed, which is an executor bug.
    pub output: Result<TransactionOutput>,
    /// Set when the execution hit a MonoMove gap, which makes the output meaningless to compare.
    pub unsupported: Option<Unsupported>,
    /// The VM error the execution reported, for diagnosing a mismatch.
    pub vm_error: Option<String>,
}

/// Executes `txn` on V2 against `state`, such as a [`crate::overrides::ReplayView`] of a patched
/// state that records the reads, and materializes its output. `Err` only if the executor could
/// not be set up.
pub(crate) fn execute<S: StateView + Sync>(
    state: &S,
    txn: &Transaction,
    aux_info: &AuxiliaryInfo,
) -> Result<V2Run> {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .ok_or_else(|| anyhow!("failed to acquire MonoMove execution guard"))?;
    let natives = production_natives();

    let module_provider = StateViewModuleProvider::new(state);
    let data_provider = StateViewResourceProvider::new(&guard, state);

    let env = AptosEnvironment::new(state);
    let usage = state.get_usage()?;
    let executor = AptosTransactionExecutor::new(
        &guard,
        natives,
        &module_provider,
        &data_provider,
        &env,
        usage,
    )
    .without_metering();

    let outcome = executor.execute_transaction(txn, aux_info);
    let unsupported = unsupported(state, &outcome);
    let vm_error = vm_error(&outcome).map(|err| format!("{:#}", err));
    let materialized = outcome
        .materialize(
            &guard,
            &data_provider,
            env.features(),
            TransactionAuxiliaryData::default(),
        )
        .map(|(output, _groups)| output);
    let unsupported = match &materialized {
        Ok(_) => unsupported,
        Err(e) => match (e.status(), e.gaps().first()) {
            // The transaction did not execute (a system transaction failed, say): the error weighs
            // no output failures, so the gap execution hit, if any, stands.
            (None, _) => unsupported,
            // Every failure writing the output is a value MonoMove cannot serialize yet, such as a
            // stored function value: that is the gap, unless execution hit one first.
            (Some(status), Some(gap)) => Some(unsupported.unwrap_or_else(|| Unsupported::Output {
                gap: gap.clone(),
                status: status.clone(),
            })),
            // A failure that is no gap is a bug, which no gap may hide, not even one the execution
            // hit before it.
            (Some(_), None) => None,
        },
    };
    Ok(V2Run {
        output: materialized.map_err(|e| anyhow!("failed to materialize V2 output: {}", e)),
        unsupported,
        vm_error,
    })
}

/// The MonoMove gap `outcome` hit, wherever in the transaction it surfaced.
fn unsupported(state: &impl StateView, outcome: &TxnOutcome) -> Option<Unsupported> {
    if let TxnOutcome::Discarded {
        reason: DiscardReason::Unsupported(what),
        ..
    } = outcome
    {
        return Some(Unsupported::Transaction(what));
    }
    let err = vm_error(outcome)?;
    // The executor always runs `versioned_prologue`, which frameworks before April 2026 lack.
    // Not finding it in a framework that has it is a bug, not a gap.
    if let Some(LoaderError::FunctionNotFound {
        address,
        module,
        name,
    }) = err.downcast_ref::<LoaderError>()
        && *address == AccountAddress::ONE
        && module == "transaction_validation"
        && name == "versioned_prologue"
        && lacks_function(state, module, name)
    {
        return Some(Unsupported::Transaction(
            "a framework without transaction_validation::versioned_prologue",
        ));
    }
    gap(err).map(Unsupported::Vm)
}

/// Whether `state` holds `0x1::<module>` and it defines no function `name`.
fn lacks_function(state: &impl StateView, module: &str, name: &str) -> bool {
    let Ok(module) = Identifier::new(module) else {
        return false;
    };
    let key = StateKey::module_id(&ModuleId::new(AccountAddress::ONE, module));
    let Ok(Some(value)) = state.get_state_value(&key) else {
        return false;
    };
    let Ok(module) = CompiledModule::deserialize(value.bytes()) else {
        return false;
    };
    !module.function_defs().iter().any(|def| {
        module
            .identifier_at(module.function_handle_at(def.function).name)
            .as_str()
            == name
    })
}

/// The VM error `outcome` carries, wherever in the transaction it surfaced.
fn vm_error(outcome: &TxnOutcome) -> Option<&VMInternalError> {
    fn failure_error(failure: &MoveExecutionFailure) -> Option<&VMInternalError> {
        match failure {
            MoveExecutionFailure::RuntimeError(err) => Some(err),
            MoveExecutionFailure::Abort { .. }
            | MoveExecutionFailure::InvalidArguments(_)
            | MoveExecutionFailure::RejectedScript(_)
            | MoveExecutionFailure::UndecryptedPayload => None,
        }
    }
    match outcome {
        TxnOutcome::Discarded { reason, .. } => match reason {
            DiscardReason::Failure { failure, .. } => failure_error(failure),
            DiscardReason::Unsupported(_)
            | DiscardReason::InvalidSignature
            | DiscardReason::KeylessValidationFailure(_)
            | DiscardReason::Deprecated(_)
            | DiscardReason::EmptyPayload
            | DiscardReason::PreExecutionCheck(_)
            | DiscardReason::InvalidTypeArgument(_)
            | DiscardReason::InvariantViolation(_) => None,
        },
        TxnOutcome::UnexpectedSystemTransactionFailure(failure) => failure_error(&failure.failure),
        TxnOutcome::Panic(err) => Some(err),
        TxnOutcome::ExecutedNoEffects(reason) => match reason {
            NoEffectsReason::BlockEpilogueFailed(failure) => failure_error(failure),
            NoEffectsReason::NothingToExecute => None,
        },
        TxnOutcome::Executed { status, .. } => match status {
            ExecutionStatus::Failure { failure, .. } => failure_error(failure),
            ExecutionStatus::Success => None,
        },
    }
}
