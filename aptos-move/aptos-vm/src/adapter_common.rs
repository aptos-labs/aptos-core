// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{AptosMoveResolver, SessionExt, SessionId};
use anyhow::Result;
use aptos_types::{
    block_metadata::BlockMetadata,
    transaction::{SignatureCheckedTransaction, SignedTransaction, Transaction, WriteSetPayload},
    vm_status::VMStatus,
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

    /// Execute a single transaction.
    fn execute_single_transaction(
        &self,
        txn: &SignatureVerifiedTransaction,
        data_cache: &impl AptosMoveResolver,
        log_context: &AdapterLogSchema,
    ) -> Result<(VMStatus, VMOutput, Option<String>), VMStatus>;
}
