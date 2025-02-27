// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::TransactionExecutable;
use crate::transaction::{user_transaction_context::MultisigPayload, EntryFunction};
use move_core_types::{account_address::AccountAddress, vm_status::VMStatus};
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

impl Multisig {
    pub fn as_multisig_payload(&self) -> MultisigPayload {
        MultisigPayload {
            multisig_address: self.multisig_address,
            entry_function_payload: self.transaction_payload.as_ref().map(
                |MultisigTransactionPayload::EntryFunction(entry)| {
                    entry.as_entry_function_payload()
                },
            ),
        }
    }

    pub fn as_transaction_executable(&self) -> TransactionExecutable {
        // TODO: See how to avoid cloning the entry function here.
        match &self.transaction_payload {
            Some(MultisigTransactionPayload::EntryFunction(entry)) => {
                TransactionExecutable::EntryFunction(entry.clone())
            },
            None => TransactionExecutable::Empty,
        }
    }
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

impl TryFrom<VMStatus> for ExecutionError {
    type Error = anyhow::Error;

    fn try_from(status: VMStatus) -> anyhow::Result<ExecutionError> {
        match status {
            VMStatus::Error {
                status_code: error, ..
            } => Ok(ExecutionError {
                error_type: String::from("VMError"),
                abort_location: String::from(""),
                error_code: error as u64,
            }),
            VMStatus::MoveAbort(abort_location, error_code) => Ok(ExecutionError {
                error_type: String::from("MoveAbort"),
                abort_location: format!("{:?}", abort_location),
                error_code,
            }),
            VMStatus::ExecutionFailure {
                status_code,
                location,
                function: _,
                code_offset: _,
                message: _,
                sub_status: _,
            } => Ok(ExecutionError {
                error_type: String::from("MoveExecutionFailure"),
                abort_location: format!("{:?}", location),
                error_code: status_code as u64,
            }),
            _ => Err(anyhow::anyhow!(
                "Unknown error from vm status cannot be converted into `ExecutionError`: {:?}",
                status
            )),
        }
    }
}
