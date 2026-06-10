// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Extracts the entry-function call to replay from a downloaded transaction.

use aptos_types::transaction::{SignedTransaction, Transaction, TransactionExecutableRef};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
};

/// Returns the inner signed transaction of a user transaction (used to derive
/// the session id), or `None` for non-user transactions.
pub fn signed_user_txn(txn: &Transaction) -> Option<&SignedTransaction> {
    if let Transaction::UserTransaction(signed) = txn {
        Some(signed)
    } else {
        None
    }
}

/// A single entry-function call, borrowed from the owning transaction.
pub struct EntryCall<'a> {
    /// Transaction sender, used for the leading `&signer` argument.
    pub sender: AccountAddress,
    pub module: &'a ModuleId,
    pub function: &'a IdentStr,
    pub ty_args: &'a [TypeTag],
    /// BCS-encoded arguments, one per non-signer parameter.
    pub args: &'a [Vec<u8>],
}

/// Returns the entry-function call to replay, or `None` if the transaction is
/// not a single-signer entry-function user transaction (scripts, multisig,
/// empty/encrypted payloads, and non-user transactions are skipped).
pub fn entry_call(txn: &Transaction) -> Option<EntryCall<'_>> {
    let Transaction::UserTransaction(signed) = txn else {
        return None;
    };
    if signed.multisig_address().is_some() {
        return None;
    }
    let entry = match signed.executable_ref().ok()? {
        TransactionExecutableRef::EntryFunction(entry) => entry,
        TransactionExecutableRef::Script(_)
        | TransactionExecutableRef::Empty
        | TransactionExecutableRef::Encrypted => return None,
    };
    Some(EntryCall {
        sender: signed.sender(),
        module: entry.module(),
        function: entry.function(),
        ty_args: entry.ty_args(),
        args: entry.args(),
    })
}
