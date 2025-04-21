// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[macro_export]
macro_rules! module_storage_error {
    ($addr:expr, $name:expr, $err:ident) => {
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

// Note:
//   The error message is formatted in the same way as by the legacy loader implementation, to
//   ensure that replay and tests work in the same way.
#[macro_export]
macro_rules! module_linker_error {
    ($addr:expr, $name:expr) => {
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
    ($addr:expr, $name:expr) => {
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
