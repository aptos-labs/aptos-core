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
    /// Execution failed with a VM error.
    RuntimeError(VMInternalError),
}

/// Whether an epilogue abort is the fee payer failing to cover the fee.
pub(crate) fn is_cant_pay_fee_abort(code: u64) -> bool {
    split_canonical(code) == (OUT_OF_RANGE, ECANT_PAY_GAS_DEPOSIT)
}
