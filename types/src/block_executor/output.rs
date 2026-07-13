// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    transaction::{TransactionAuxiliaryData, TransactionOutput, TransactionStatus},
    write_set::WriteSet,
};
use move_core_types::vm_status::StatusCode;
use std::fmt::Debug;

/// The final, materialized transaction output the block executor stores in its
/// results.
pub trait CommittedTransactionOutput: Send + Debug {
    /// Fee statement of the (kept) transaction.
    fn fee_statement(&self) -> FeeStatement;

    /// Whether the transaction emitted a new epoch event.
    fn has_new_epoch_event(&self) -> bool;

    /// Whether this is a placeholder for a not-yet-materialized (retry) slot.
    /// Placeholders are created via [`Self::retry_placeholder`] and replaced on
    /// materialization, so a slot that is still a placeholder was skipped.
    fn is_retry(&self) -> bool;

    /// Whether the transaction was kept and succeeded.
    fn is_success(&self) -> bool;

    /// Placeholder for a results slot that has not been materialized.
    fn retry() -> Self;

    /// Placeholder for a discarded transaction.
    fn discard(discard_code: StatusCode) -> Self;
}

impl CommittedTransactionOutput for TransactionOutput {
    fn fee_statement(&self) -> FeeStatement {
        self.try_extract_fee_statement()
            .ok()
            .flatten()
            .unwrap_or_else(FeeStatement::zero)
    }

    fn has_new_epoch_event(&self) -> bool {
        self.events().iter().any(ContractEvent::is_new_epoch_event)
    }

    fn is_retry(&self) -> bool {
        self.status().is_retry()
    }

    fn is_success(&self) -> bool {
        self.status()
            .as_kept_status()
            .is_ok_and(|status| status.is_success())
    }

    fn retry() -> Self {
        TransactionOutput::new(
            WriteSet::default(),
            vec![],
            0,
            TransactionStatus::Retry,
            TransactionAuxiliaryData::None,
        )
    }

    fn discard(discard_code: StatusCode) -> Self {
        TransactionOutput::new(
            WriteSet::default(),
            vec![],
            0,
            TransactionStatus::Discard(discard_code),
            TransactionAuxiliaryData::None,
        )
    }
}
