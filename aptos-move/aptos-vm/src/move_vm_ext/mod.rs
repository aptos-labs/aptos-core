// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! MoveVM and Session wrapped, to make sure Aptos natives and extensions are always installed and
//! taken care of after session finish.
mod resolver;
pub mod session;
mod vm;
pub(crate) mod write_op_converter;

pub use crate::move_vm_ext::{
    resolver::{AptosMoveResolver, AsExecutorView, ResourceGroupResolver},
    session::{convert_modules_into_write_ops, SessionExt},
    vm::{GenesisMoveVm, GenesisRuntimeBuilder, MoveVmExt},
};
use aptos_types::state_store::state_key::StateKey;
pub use aptos_types::transaction::user_transaction_context::UserTransactionContext;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, vm_status::StatusCode,
};
pub use session::session_id::SessionId;

pub(crate) fn resource_state_key(
    address: &AccountAddress,
    tag: &StructTag,
) -> PartialVMResult<StateKey> {
    StateKey::resource(address, tag).map_err(|e| {
        PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)
            .with_message(format!("Failed to serialize struct tag: {}", e))
    })
}
