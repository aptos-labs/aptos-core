// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines the gas parameters for the table extension.

use crate::gas_schedule::NativeGasParameters;
use aptos_gas_algebra::{InternalGas, InternalGasPerByte};

crate::gas_schedule::macros::define_gas_parameters!(
    TableGasParameters,
    "table",
    NativeGasParameters => .table,
    [
        // These are dummy value, they copied from storage gas in aptos-core/aptos-vm/src/aptos_vm_impl.rs
        [common_load_base_legacy: InternalGas, "common.load.base", 3023850],
        [common_load_base_new: InternalGas, { 7.. => "common.load.base_new" }, 3023850],
        [common_load_per_byte: InternalGasPerByte, "common.load.per_byte", 1510],
        [common_load_failure: InternalGas, "common.load.failure", 0],

        [new_table_handle_base: InternalGas, "new_table_handle.base", 36760],

        [add_box_base: InternalGas, "add_box.base", 44110],
        [add_box_per_byte_serialized: InternalGasPerByte, "add_box.per_byte_serialized", 360],

        [borrow_box_base: InternalGas, "borrow_box.base", 44110],
        [borrow_box_per_byte_serialized: InternalGasPerByte, "borrow_box.per_byte_serialized", 360],

        [contains_box_base: InternalGas, "contains_box.base", 44110],
        [contains_box_per_byte_serialized: InternalGasPerByte, "contains_box.per_byte_serialized", 360],

        [remove_box_base: InternalGas, "remove_box.base", 44110],
        [remove_box_per_byte_serialized: InternalGasPerByte, "remove_box.per_byte_serialized", 360],

        [destroy_empty_box_base: InternalGas, "destroy_empty_box.base", 44110],

        [drop_unchecked_box_base: InternalGas, "drop_unchecked_box.base", 3670],
    ]
);
