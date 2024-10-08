// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! panic_error {
    ($msg:expr) => {{
        println!("[Error] panic detected: {}", $msg);
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
        )
        .with_message(format!("Panic detected: {:?}", $msg))
    }};
}

#[macro_export]
macro_rules! module_storage_error {
    ($addr:ident, $name:ident, $err:ident) => {
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::STORAGE_ERROR,
        )
        .with_message(format!(
            "Unexpected storage error for module {}::{}: {:?}",
            $addr, $name, $err
        ))
        .finish(move_binary_format::errors::Location::Undefined)
    };
}

// TODO(loader_v2):
//   The error message is formatted in the same way as V1, to ensure that replay and tests work in
//   the same way, but ideally we should use proper formatting here.
#[macro_export]
macro_rules! module_linker_error {
    ($addr:ident, $name:ident) => {
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::LINKER_ERROR,
        )
        .with_message(format!(
            "Linker Error: Module {}::{} doesn't exist",
            $addr.to_hex(),
            $name
        ))
        .finish(move_binary_format::errors::Location::Undefined)
    };
}

#[macro_export]
macro_rules! module_cyclic_dependency_error {
    ($addr:ident, $name:ident) => {
        move_binary_format::errors::PartialVMError::new(
            move_core_types::vm_status::StatusCode::CYCLIC_MODULE_DEPENDENCY,
        )
        .with_message(format!(
            "Module {}::{} forms a cyclic dependency",
            $addr, $name
        ))
        .finish(move_binary_format::errors::Location::Module(
            move_core_types::language_storage::ModuleId::new(*$addr, $name.to_owned()),
        ))
    };
}
