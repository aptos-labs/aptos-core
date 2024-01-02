// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the gas parameters for the table extension.

use crate::gas_schedule::NativeGasParameters;
use aptos_gas_algebra::{InternalGas, InternalGasPerByte};

crate::gas_schedule::macros::define_gas_parameters!(
    TableGasParameters,
    "table",
    NativeGasParameters => .table,
    [
        // These are dummy value, they copied from storage gas in aptos-core/aptos-vm/src/aptos_vm_impl.rs
        [common_load_base_legacy: InternalGas, "common.load.base", 8000],
        [common_load_base_new: InternalGas, { 7.. => "common.load.base_new" }, 8000],
        [common_load_per_byte: InternalGasPerByte, "common.load.per_byte", 1000],
        [common_load_failure: InternalGas, "common.load.failure", 0],

        [new_table_handle_base: InternalGas, "new_table_handle.base", 20000],

        [add_box_base: InternalGas, "add_box.base", 24000],
        [add_box_per_byte_serialized: InternalGasPerByte, "add_box.per_byte_serialized", 200],

        [borrow_box_base: InternalGas, "borrow_box.base", 24000],
        [borrow_box_per_byte_serialized: InternalGasPerByte, "borrow_box.per_byte_serialized", 200],

        [contains_box_base: InternalGas, "contains_box.base", 24000],
        [contains_box_per_byte_serialized: InternalGasPerByte, "contains_box.per_byte_serialized", 200],

        [remove_box_base: InternalGas, "remove_box.base", 24000],
        [remove_box_per_byte_serialized: InternalGasPerByte, "remove_box.per_byte_serialized", 200],

        [destroy_empty_box_base: InternalGas, "destroy_empty_box.base", 24000],

        [drop_unchecked_box_base: InternalGas, "drop_unchecked_box.base", 2000],
    ]
);
