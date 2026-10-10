// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_keyless_validation::KeylessValidationError;
use aptos_types::{
    error::{split_canonical, OUT_OF_RANGE},
    transaction::{validation::ECANT_PAY_GAS_DEPOSIT, TransactionStatus},
};
use mono_move_core::VMInternalError;
use mono_move_output::gap::Gap;
use mono_move_runtime::{
    error::{RuntimeError, RuntimeInvariantViolation},
    RuntimeStatus,
};
use move_core_types::vm_status::AbortLocation;
use thiserror::Error;

/// Every reason a transaction's effects could not be rendered into a
/// `TransactionOutput`. An executor bug, unless a value hit a MonoMove gap; the
/// reasons are diagnostics.
#[derive(Debug, Error)]
#[error("failed to materialize the transaction output: {}", .reasons.join("; "))]
pub struct MaterializationError {
    reasons: Vec<String>,
    gaps: Vec<Gap>,
    status: Option<TransactionStatus>,
}

impl MaterializationError {
    /// Sorts the reasons, so that what is reported does not depend on the order
    /// the failures were hit in.
    pub fn new(mut reasons: Vec<String>) -> Self {
        reasons.sort();
        Self {
            reasons,
            gaps: vec![],
            status: None,
        }
    }

    /// The failures of an executed transaction's output, which would have
    /// committed with `status`. Its gaps are kept only if every failure is one:
    /// any other failure is an executor bug, which a gap must not hide.
    pub(crate) fn of_executed(
        failures: Vec<MaterializationFailure>,
        status: TransactionStatus,
    ) -> Self {
        let only_gaps = failures.iter().all(|failure| failure.gap.is_some());
        let (reasons, gaps): (Vec<_>, Vec<_>) = failures
            .into_iter()
            .map(|failure| (failure.reason, failure.gap))
            .unzip();
        let mut gaps: Vec<Gap> = if only_gaps {
            gaps.into_iter().flatten().collect()
        } else {
            vec![]
        };
        // By message, then kind, so that the order (and which gap comes first) does not depend on
        // the order the failures were found in, and duplicates are adjacent for `dedup`.
        gaps.sort_by_cached_key(|gap| (gap.message.clone(), format!("{:?}", gap.kind)));
        gaps.dedup();
        Self {
            gaps,
            status: Some(status),
            ..Self::new(reasons)
        }
    }

    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }

    /// The MonoMove gaps the values hit, e.g. a written function value, when
    /// they are the only failures. Empty when any failure is an executor bug.
    pub fn gaps(&self) -> &[Gap] {
        &self.gaps
    }

    /// The status the transaction executed with, final before its output was
    /// materialized; `None` if it did not execute.
    pub fn status(&self) -> Option<&TransactionStatus> {
        self.status.as_ref()
    }
}

/// One way an executed transaction's output failed to materialize.
pub(crate) struct MaterializationFailure {
    pub(crate) reason: String,
    /// Set when the failure is a MonoMove gap, e.g. a value it cannot serialize
    /// yet, rather than an executor bug.
    pub(crate) gap: Option<Gap>,
}

impl From<String> for MaterializationFailure {
    fn from(reason: String) -> Self {
        Self { reason, gap: None }
    }
}

/// Why a transaction was discarded: it produced no side effects, and only its
/// rejection reason is observable.
#[derive(Debug)]
pub enum DiscardReason {
    /// The transaction's signature did not verify.
    InvalidSignature,
    /// A keyless authenticator did not validate.
    KeylessValidationFailure(KeylessValidationError),
    /// A transaction shape this executor does not support yet.
    Unsupported(&'static str),
    /// A payload or feature that no VM supports anymore.
    Deprecated(&'static str),
    /// A non-multisig transaction carries no executable.
    EmptyPayload,
    /// A pre-execution check failed.
    PreExecutionCheck(PreExecutionCheckFailure),
    /// A type argument failed to resolve.
    InvalidTypeArgument(String),
    Failure {
        stage: ExecutionStage,
        failure: MoveExecutionFailure,
    },
    /// An executor-internal invariant violation.
    InvariantViolation(String),
}

/// Why a transaction committed without side effects.
#[derive(Debug)]
pub enum NoEffectsReason {
    /// The transaction had nothing to execute.
    NothingToExecute,
    /// The block epilogue failed. AptosVM commits an empty success rather than
    /// aborting the block.
    BlockEpilogueFailed(MoveExecutionFailure),
}

/// A system transaction failed. System code is expected to always succeed, and
/// if it fails, it means there is a bug in the executor or the framework.
///
/// When this happens, the block executor has no choice but to abort the whole block.
#[derive(Debug)]
pub struct SystemTxnFailure {
    /// The framework call that failed.
    pub call: &'static str,
    pub failure: MoveExecutionFailure,
}

/// The pre-execution bound a transaction violated. Sizes are in bytes, gas in
/// gas units, prices in octas per gas unit.
#[derive(Debug, Error)]
pub enum PreExecutionCheckFailure {
    #[error("transaction size {size} exceeds the maximum {max}")]
    TransactionTooLarge { size: u64, max: u64 },
    #[error("max gas amount {max_gas} exceeds the bound {bound}")]
    GasBudgetAboveBound { max_gas: u64, bound: u64 },
    #[error("max gas amount {max_gas} is below the transaction's base cost {min}")]
    GasBudgetBelowIntrinsicCost { max_gas: u64, min: u64 },
    #[error("gas unit price {price} is below the minimum {min}")]
    GasPriceBelowMinimum { price: u64, min: u64 },
    #[error("gas unit price {price} is below the encrypted-transaction minimum {min}")]
    EncryptedGasPriceBelowMinimum { price: u64, min: u64 },
    #[error("gas unit price {price} is below the raised-limits minimum {min}")]
    HighLimitGasPriceBelowMinimum { price: u64, min: u64 },
    #[error("limits multiplier {percent}% must be above {min}% and at most {max}%")]
    InvalidLimitsMultiplier { percent: u64, min: u64, max: u64 },
    #[error("an approved governance script may not request raised limits")]
    LimitsRequestOnGovernanceScript,
    #[error("gas unit price {price} is above the maximum {max}")]
    GasPriceAboveMaximum { price: u64, max: u64 },
}

/// Which Move call the transaction was in when it failed.
#[derive(Clone, Copy, Debug)]
pub enum ExecutionStage {
    Prologue,
    Payload,
    /// The epilogue after a payload that succeeded.
    Epilogue,
    /// The epilogue after a payload that was rolled back.
    EpilogueAfterRollback,
    /// The epilogue rerun after the first one failed.
    EpilogueRetry,
}

/// How an executed transaction concluded. Every variant commits on-chain and is
/// charged the fee.
#[derive(Debug)]
pub enum ExecutionStatus {
    Success,
    /// The payload or the epilogue failed; the payload's effects were dropped.
    Failure {
        stage: ExecutionStage,
        failure: MoveExecutionFailure,
    },
}

/// Why a transaction's call was rejected: the function is not one a
/// transaction may call, or the arguments do not fit it. In the order they are
/// checked.
#[derive(Debug)]
pub enum InvalidArguments {
    /// The function is a native, which a transaction may not call directly.
    NativeEntryFunction,
    /// The function is not an `entry` function.
    NotEntryFunction,
    /// The function returns values.
    ReturnsValues,
    /// A signer parameter follows a non-signer one.
    SignerAfterArgument,
    /// A parameter has a type a transaction argument cannot fill.
    DisallowedParameterType,
    /// The argument count does not match the function's parameters.
    ArgumentCountMismatch,
    /// The signer count does not match the function's signer parameters.
    SignerCountMismatch,
    /// An argument's bytes do not decode to its parameter's type.
    UndecodableArgument,
    /// A `String` argument is not valid UTF-8.
    MalformedString,
    /// An `Object<T>` argument names an address holding no object.
    ObjectDoesNotExist,
    /// An `Object<T>` argument names an object holding no `T`.
    ObjectLacksResource,
}

/// Why a script was refused before running.
#[derive(Debug)]
pub enum ScriptRejection {
    /// Its compiler marked it unstable, which mainnet does not run.
    UnstableOnMainnet,
    /// It emits events, which scripts may not.
    EmitsEvents,
}

/// How Move execution failed, whether it was the prologue, the payload, the
/// epilogue, or the transaction as a whole. What a failure means for the
/// transaction is the driver's call.
#[derive(Debug)]
pub enum MoveExecutionFailure {
    /// Execution reached a Move abort.
    Abort {
        code: u64,
        message: Option<String>,
        location: AbortLocation,
    },
    /// The transaction's arguments were rejected.
    InvalidArguments(InvalidArguments),
    /// The transaction's script was refused before running.
    RejectedScript(ScriptRejection),
    /// The payload is still encrypted: decryption failed before execution.
    UndecryptedPayload,
    /// Execution failed with a VM error.
    RuntimeError(VMInternalError),
}

/// An error that should not be reachable, as a VM error.
pub(crate) fn invariant_violation(detail: impl Into<String>) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(detail.into()),
    ))
}

/// Reduces a completed call to success or the abort it ended in.
pub(crate) fn call_result(status: RuntimeStatus) -> Result<(), MoveExecutionFailure> {
    match status {
        RuntimeStatus::Success => Ok(()),
        RuntimeStatus::Aborted {
            code,
            message,
            location,
            ..
        } => Err(MoveExecutionFailure::Abort {
            code,
            message,
            location,
        }),
    }
}

/// Whether an epilogue abort is the fee payer failing to cover the fee.
pub(crate) fn is_cant_pay_fee_abort(code: u64) -> bool {
    split_canonical(code) == (OUT_OF_RANGE, ECANT_PAY_GAS_DEPOSIT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_output::gap::GapKind;

    fn gap(message: &str) -> Gap {
        Gap {
            kind: GapKind::Other,
            message: message.to_string(),
        }
    }

    fn failure(reason: &str, gap: Option<Gap>) -> MaterializationFailure {
        MaterializationFailure {
            reason: reason.to_string(),
            gap,
        }
    }

    #[test]
    fn gaps_are_kept_only_when_every_failure_is_one() {
        let status = TransactionStatus::Keep(aptos_types::transaction::ExecutionStatus::Success);
        let all = MaterializationError::of_executed(
            vec![failure("a", Some(gap("f"))), failure("b", Some(gap("f")))],
            status.clone(),
        );
        assert_eq!(all.gaps(), &[gap("f")]);
        assert_eq!(all.status(), Some(&status));
        // A failure that is not a gap is an executor bug, which the gap must not hide.
        let mixed = MaterializationError::of_executed(
            vec![failure("a", Some(gap("f"))), failure("b", None)],
            status,
        );
        assert!(mixed.gaps().is_empty());
        assert_eq!(mixed.reasons(), &["a".to_string(), "b".to_string()]);
    }
}
