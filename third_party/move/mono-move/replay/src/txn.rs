// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! What a transaction is, for deciding whether it is replayed and for reports.

use aptos_types::transaction::{Transaction, TransactionExecutableRef};
use mono_move_replay_common::label;

/// Whether the comparison replays `txn`: user, block-metadata and block-epilogue transactions.
pub(crate) fn is_replayed(txn: &Transaction) -> bool {
    match txn {
        Transaction::UserTransaction(_)
        | Transaction::BlockMetadata(_)
        | Transaction::BlockMetadataExt(_)
        | Transaction::BlockEpilogue(_) => true,
        Transaction::GenesisTransaction(_)
        | Transaction::StateCheckpoint(_)
        | Transaction::ValidatorTransaction(_) => false,
    }
}

/// What kind of transaction `txn` is, for reports.
pub(crate) fn txn_kind(txn: &Transaction) -> &'static str {
    match txn {
        Transaction::UserTransaction(signed) => {
            let multisig = signed.payload().is_multisig();
            match (signed.executable_ref(), multisig) {
                (_, true) => "user/multisig",
                (Ok(TransactionExecutableRef::EntryFunction(_)), false) => "user/entry_function",
                (Ok(TransactionExecutableRef::Script(_)), false) => "user/script",
                (Ok(TransactionExecutableRef::Empty), false) => "user/empty",
                (Ok(TransactionExecutableRef::Encrypted), false) => "user/encrypted",
                (Err(_), false) => "user/unknown",
            }
        },
        Transaction::BlockMetadata(_) => "block_metadata",
        Transaction::BlockMetadataExt(_) => "block_metadata_ext",
        Transaction::BlockEpilogue(_) => "block_epilogue",
        Transaction::GenesisTransaction(_) => "genesis",
        Transaction::StateCheckpoint(_) => "state_checkpoint",
        Transaction::ValidatorTransaction(_) => "validator",
    }
}

/// What the transaction runs, for reports: an entry function's `module::function<type args>`, the
/// block-metadata or block-epilogue variant, or otherwise its [`txn_kind`].
pub(crate) fn label(txn: &Transaction) -> String {
    match txn {
        Transaction::UserTransaction(signed) => match signed.executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry)) => label::entry_function(entry),
            Ok(
                TransactionExecutableRef::Script(_)
                | TransactionExecutableRef::Empty
                | TransactionExecutableRef::Encrypted,
            )
            | Err(_) => txn_kind(txn).to_string(),
        },
        Transaction::BlockMetadata(_) => "block_metadata".to_string(),
        Transaction::BlockMetadataExt(bme) => label::block_metadata_ext(bme).to_string(),
        Transaction::BlockEpilogue(payload) => label::block_epilogue(payload).to_string(),
        Transaction::GenesisTransaction(_)
        | Transaction::StateCheckpoint(_)
        | Transaction::ValidatorTransaction(_) => txn_kind(txn).to_string(),
    }
}
