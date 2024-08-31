// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

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

// TODO(loader_v2): The error message is formatted in the same way as V1, to
//                  make tests pass, but ideally we should improve this.
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
