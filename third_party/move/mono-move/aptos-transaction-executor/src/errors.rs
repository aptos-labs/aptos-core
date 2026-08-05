// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::{
    error::{split_canonical, OUT_OF_RANGE},
    transaction::validation::ECANT_PAY_GAS_DEPOSIT,
};
use mono_move_core::VMInternalError;
use move_core_types::vm_status::AbortLocation;
use thiserror::Error;

/// Every reason a transaction's effects could not be rendered into a
/// `TransactionOutput`. Always an executor bug; the reasons are diagnostics.
#[derive(Debug, Error)]
#[error("failed to materialize the transaction output: {}", .0.join("; "))]
pub struct MaterializationError(Vec<String>);

impl MaterializationError {
    /// Sorts the reasons, so that what is reported does not depend on the order
    /// the failures were hit in.
    pub fn new(mut reasons: Vec<String>) -> Self {
        reasons.sort();
        Self(reasons)
    }

    pub fn reasons(&self) -> &[String] {
        &self.0
    }
}

/// Why a transaction was discarded: it produced no side effects, and only its
/// rejection reason is observable.
#[derive(Debug)]
pub enum DiscardReason {
    /// A transaction shape this executor does not support yet.
    Unsupported(&'static str),
    /// A type argument failed to resolve.
    InvalidTypeArgument(String),
    Prologue(PrologueFailure),
    /// The epilogue misbehaved. One versioned Move epilogue runs from up to
    /// three places, and `run` says which one failed.
    Epilogue {
        run: &'static str,
        failure: EpilogueFailure,
    },
    /// An executor-internal invariant violation.
    InvariantViolation(String),
}

/// How an executed transaction concluded. Any conclusion other than success
/// still charges the fee and bumps the sequence number.
#[derive(Debug)]
pub enum ExecutionStatus {
    Success,
    /// A Move abort, raised either by the payload or by the epilogue's balance
    /// check; the payload's effects were dropped.
    Abort {
        code: u64,
        message: Option<String>,
        location: AbortLocation,
    },
    /// The payload failed with a VM error; its effects were dropped.
    Failure(VMInternalError),
    /// The epilogue failed some way it must not after a successful payload, and
    /// rerunning it against the state the prologue left succeeded. The payload's
    /// effects were dropped and the fee still charged. The fee abort is
    /// legitimate and becomes `Abort` instead.
    RecoveredEpilogueFailure(EpilogueFailure),
}

/// A prologue failure; always discards the transaction.
#[derive(Debug)]
pub enum PrologueFailure {
    /// The prologue aborted: the transaction failed validation.
    Abort { code: u64, message: Option<String> },
    /// The prologue must not fail any other way.
    Unexpected(String),
}

/// A payload execution failure. An abort always commits (charging the fee);
/// whether a VM error commits or discards follows the legacy keep/discard
/// rules.
#[derive(Debug)]
pub(crate) enum PayloadFailure {
    /// The payload executed a Move abort.
    Abort { code: u64, message: Option<String> },
    /// The payload failed with a VM error.
    Internal(VMInternalError),
}

/// An epilogue failure. The fee payer failing to cover the fee is the only
/// legitimate one, which the driver recognizes by the abort code.
#[derive(Debug)]
pub enum EpilogueFailure {
    /// The epilogue executed a Move abort.
    Abort { code: u64, message: Option<String> },
    /// The epilogue failed with a VM error.
    Internal(VMInternalError),
}

/// Whether an epilogue abort is the fee payer failing to cover the fee.
pub(crate) fn is_cant_pay_fee_abort(code: u64) -> bool {
    split_canonical(code) == (OUT_OF_RANGE, ECANT_PAY_GAS_DEPOSIT)
}
