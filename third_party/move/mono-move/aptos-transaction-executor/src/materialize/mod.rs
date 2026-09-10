// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Converts the transaction's output into both the storage format (`TransactionOutput`)
//! and the legacy `VMStatus`.

mod txn_output;
mod vm_status;

pub(crate) use txn_output::{discarded_output, empty_success_output, executed_output};
pub(crate) use vm_status::{discard_to_vm_status, executed_vm_status};
