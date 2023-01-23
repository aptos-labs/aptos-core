// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::EntryFunction;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

/// A multisig transaction that allows an owner of a multisig account to execute a pre-approved
/// transaction as the multisig account.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Multisig {
    pub multisig_address: AccountAddress,

    // Transaction payload is optional if already stored on chain.
    pub transaction_payload: Option<MultisigTransactionPayload>,
}

// We use an enum here for extensibility so we can add Script payload support
// in the future for example.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum MultisigTransactionPayload {
    EntryFunction(EntryFunction),
}

/// Contains information about execution failure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionError {
    // The module where the error occurred.
    pub abort_location: String,
    pub error_type: String,
    // The detailed error code explaining which error occurred.
    pub error_code: u64,
}
