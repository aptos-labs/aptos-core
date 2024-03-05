// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_metadata::TransactionMetadata;

pub mod abort_hook;
pub mod epilogue;
pub mod prologue;
pub mod user;

#[derive(Clone, Copy)]
pub struct Context<'l> {
    txn_meta: &'l TransactionMetadata,
}
