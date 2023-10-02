// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{AptosMoveResolver, SessionExt, SessionId};
use anyhow::Result;
use aptos_types::{
    transaction::{
        SignatureCheckedTransaction, SignatureVerifiedTransaction, SignedTransaction,
        TransactionStatus,
    },
    vm_status::{StatusCode, VMStatus},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::output::VMOutput;

/// This trait describes the VM adapter's interface.
/// TODO: bring more of the execution logic in aptos_vm into this file.
pub trait VMAdapter {
    /// Creates a new Session backed by the given storage.
    /// TODO: this doesn't belong in this trait. We should be able to remove
    /// this after redesigning cache ownership model.
    fn new_session<'r>(
        &self,
        remote: &'r impl AptosMoveResolver,
        session_id: SessionId,
    ) -> SessionExt<'r, '_>;

    /// Checks the signature of the given signed transaction and returns
    /// `Ok(SignatureCheckedTransaction)` if the signature is valid.
    fn check_signature(txn: SignedTransaction) -> Result<SignatureCheckedTransaction>;

    /// Check if the transaction format is supported.
    fn check_transaction_format(&self, txn: &SignedTransaction) -> Result<(), VMStatus>;

    /// Runs the prologue for the given transaction.
    fn run_prologue(
        &self,
        session: &mut SessionExt,
        resolver: &impl AptosMoveResolver,
        transaction: &SignedTransaction,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus>;

    /// TODO: maybe remove this after more refactoring of execution logic.
    fn should_restart_execution(output: &VMOutput) -> bool;

    /// Execute a single transaction.
    fn execute_single_transaction(
        &self,
        txn: &SignatureVerifiedTransaction,
        data_cache: &impl AptosMoveResolver,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput, Option<String>), VMStatus>;

    fn validate_signature_checked_transaction(
        &self,
        session: &mut SessionExt,
        resolver: &impl AptosMoveResolver,
        transaction: &SignedTransaction,
        allow_too_new: bool,
        log_context: &AdapterLogSchema,
    ) -> Result<(), VMStatus> {
        self.check_transaction_format(transaction)?;

        let prologue_status = self.run_prologue(session, resolver, transaction, log_context);
        match prologue_status {
            Err(err)
                if !allow_too_new || err.status_code() != StatusCode::SEQUENCE_NUMBER_TOO_NEW =>
            {
                Err(err)
            },
            _ => Ok(()),
        }
    }
}

pub(crate) fn discard_error_vm_status(err: VMStatus) -> (VMStatus, VMOutput) {
    let vm_status = err.clone();
    (vm_status, discard_error_output(err.status_code()))
}

pub(crate) fn discard_error_output(err: StatusCode) -> VMOutput {
    // Since this transaction will be discarded, no writeset will be included.
    VMOutput::empty_with_status(TransactionStatus::Discard(err))
}
