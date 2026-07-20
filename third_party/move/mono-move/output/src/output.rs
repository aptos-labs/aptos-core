// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove effects → Aptos [`TransactionOutput`].

use crate::{error::OutputResult, events::to_contract_events};
use aptos_types::{
    transaction::{
        AbortInfo, ExecutionStatus, TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
    },
    write_set::WriteSet,
};
use mono_move_core::{value_layout::LayoutProvider, ExecutionErrorKind, ExecutionResult};
use mono_move_runtime::SessionEffects;
use move_core_types::vm_status::AbortLocation;

/// Builds the [`TransactionOutput`] for a finished MonoMove transaction from its
/// [`SessionEffects`] and run `result`. `layouts` serializes the written values
/// and event payloads; `gas_used` and `auxiliary_data` are supplied by the caller.
///
/// A successful run commits its writes and events; an abort or failure commits
/// an empty write set with the mapped status.
///
/// TODO(cleanup): decide where this belongs — maybe a `materialize` method on
/// `SessionEffects` rather than a free function taking `&SessionEffects`.
pub fn to_transaction_output(
    effects: &SessionEffects,
    layouts: &impl LayoutProvider,
    result: ExecutionResult,
    gas_used: u64,
    auxiliary_data: TransactionAuxiliaryData,
) -> OutputResult<TransactionOutput> {
    let (write_set, events) = match &result {
        ExecutionResult::Success => {
            let write_set = effects.write_set(layouts)?;
            // SAFETY: the effects' frozen heap (which event payloads point into) is intact.
            let events = unsafe { to_contract_events(&effects.extensions, layouts) }?;
            (write_set, events)
        },
        ExecutionResult::Aborted { .. } | ExecutionResult::Failed(_) => {
            (WriteSet::default(), Vec::new())
        },
    };
    Ok(TransactionOutput::new(
        write_set,
        events,
        gas_used,
        to_transaction_status(result),
        auxiliary_data,
    ))
}

/// Maps a MonoMove [`ExecutionResult`] to an Aptos [`TransactionStatus`].
///
/// The abort location is a placeholder (the runtime status carries none yet),
/// and non-abort failures other than out-of-gas collapse to a miscellaneous
/// error.
pub fn to_transaction_status(result: ExecutionResult) -> TransactionStatus {
    let status = match result {
        ExecutionResult::Success => ExecutionStatus::Success,
        // TODO(completeness): report the real abort location (module or script)
        // once the runtime status carries it, rather than always Script.
        ExecutionResult::Aborted { code, message } => ExecutionStatus::MoveAbort {
            location: AbortLocation::Script,
            code,
            info: message.map(|description| AbortInfo {
                // TODO(completeness): `reason_name` is filled from the module error map by a later
                // injection pass (see `inject_abort_info_if_available`); the new VM still needs to
                // wire that up.
                reason_name: String::new(),
                description,
            }),
        },
        // TODO(completeness): figure out how to represent MonoMove failures as
        // Aptos statuses — e.g. dedicated StatusCodes — instead of collapsing
        // everything but out-of-gas to a miscellaneous error.
        ExecutionResult::Failed(err) => match err.kind {
            ExecutionErrorKind::OutOfGas => ExecutionStatus::OutOfGas,
            ExecutionErrorKind::RuntimeLimitExceeded
            | ExecutionErrorKind::InvalidOperation
            | ExecutionErrorKind::LinkingError
            | ExecutionErrorKind::InvariantViolation
            | ExecutionErrorKind::Placeholder => ExecutionStatus::MiscellaneousError(None),
        },
    };
    TransactionStatus::Keep(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_core::ExecutionError;

    #[test]
    fn status_mapping() {
        assert!(matches!(
            to_transaction_status(ExecutionResult::Success),
            TransactionStatus::Keep(ExecutionStatus::Success)
        ));
        assert!(matches!(
            to_transaction_status(ExecutionResult::Aborted {
                code: 42,
                message: None
            }),
            TransactionStatus::Keep(ExecutionStatus::MoveAbort { code: 42, .. })
        ));
        assert!(matches!(
            to_transaction_status(ExecutionResult::Failed(ExecutionError {
                kind: ExecutionErrorKind::OutOfGas,
                message: String::new(),
            })),
            TransactionStatus::Keep(ExecutionStatus::OutOfGas)
        ));
    }
}
