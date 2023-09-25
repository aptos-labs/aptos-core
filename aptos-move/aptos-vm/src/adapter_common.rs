// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{AptosMoveResolver, SessionExt, SessionId};
use anyhow::Result;
use aptos_types::{
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, SignatureCheckedTransaction,
        SignedTransaction, TransactionStatus,
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
    fn check_signature(&self, txn: SignedTransaction) -> Result<SignatureCheckedTransaction>;

    /// Check if the transaction format is supported.
    fn check_transaction_format(&self, txn: &SignedTransaction) -> Result<(), VMStatus>;

    /// TODO: maybe remove this after more refactoring of execution logic.
    fn should_restart_execution(output: &VMOutput) -> bool;

    /// Execute a single transaction.
    fn execute_single_transaction(
        &self,
        txn: &SignatureVerifiedTransaction,
        data_cache: &impl AptosMoveResolver,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput, Option<String>), VMStatus>;
}

pub(crate) fn discard_error_vm_status(err: VMStatus) -> (VMStatus, VMOutput) {
    let vm_status = err.clone();
    (vm_status, discard_error_output(err.status_code()))
}

pub(crate) fn discard_error_output(err: StatusCode) -> VMOutput {
    // Since this transaction will be discarded, no write set will be included.
    VMOutput::empty_with_status(TransactionStatus::Discard(err))
}
