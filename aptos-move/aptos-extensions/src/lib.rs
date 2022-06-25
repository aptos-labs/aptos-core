// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble};

/// The native transaction context extension. This needs to be attached to the NativeContextExtensions
/// value which is passed into session functions, so its accessible from natives of this
/// extension.
#[derive(Tid)]
pub struct NativeTransactionContext {
    // TODO: here we can pass more information from transaction, e.g sequence
    // number, etc. For now, just use the hash of transaction.
    txn_hash: u128,
}

impl NativeTransactionContext {
    pub fn new(txn_hash: u128) -> Self {
        Self { txn_hash, }
    }

    pub fn txn_hash(&self) -> u128 {
        self.txn_hash
    }
}
