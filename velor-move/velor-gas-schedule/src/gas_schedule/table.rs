// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the gas parameters for the table extension.

use crate::gas_schedule::NativeGasParameters;
use velor_gas_algebra::{InternalGas, InternalGasPerByte};

crate::gas_schedule::macros::define_gas_parameters!(
    TableGasParameters,
    "table",
    NativeGasParameters => .table,
    [
        // These are dummy value, they copied from storage gas in velor-core/velor-vm/src/velor_vm_impl.rs
        [common_load_base_legacy: InternalGas, "common.load.base", 302385],
        [common_load_base_new: InternalGas, { 7.. => "common.load.base_new" }, 302385],
        [common_load_per_byte: InternalGasPerByte, "common.load.per_byte", 151],
        [common_load_failure: InternalGas, "common.load.failure", 0],

        [new_table_handle_base: InternalGas, "new_table_handle.base", 3676],

        [add_box_base: InternalGas, "add_box.base", 4411],
        [add_box_per_byte_serialized: InternalGasPerByte, "add_box.per_byte_serialized", 36],

        [borrow_box_base: InternalGas, "borrow_box.base", 4411],
        [borrow_box_per_byte_serialized: InternalGasPerByte, "borrow_box.per_byte_serialized", 36],

        [contains_box_base: InternalGas, "contains_box.base", 4411],
        [contains_box_per_byte_serialized: InternalGasPerByte, "contains_box.per_byte_serialized", 36],

        [remove_box_base: InternalGas, "remove_box.base", 4411],
        [remove_box_per_byte_serialized: InternalGasPerByte, "remove_box.per_byte_serialized", 36],

        [destroy_empty_box_base: InternalGas, "destroy_empty_box.base", 4411],

        [drop_unchecked_box_base: InternalGas, "drop_unchecked_box.base", 367],
    ]
);
