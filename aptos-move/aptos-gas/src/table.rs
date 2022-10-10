// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::gas_meter::EXECUTION_GAS_MULTIPLIER as MUL;
use move_table_extension::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "table", [
    // Note(Gas): These are legacy parameters for loading from storage so they do not
    //            need to be multiplied.
    [.common.load_base, "common.load.base", 8000],
    [.common.load_per_byte, "common.load.per_byte", 1000],
    [.common.load_failure, "common.load.failure", 0],

    [.new_table_handle.base, "new_table_handle.base", 1000 * MUL],

    [.add_box.base, "add_box.base", 1200 * MUL],
    [.add_box.per_byte_serialized, "add_box.per_byte_serialized", 10 * MUL],

    [.borrow_box.base, "borrow_box.base", 1200 * MUL],
    [.borrow_box.per_byte_serialized, "borrow_box.per_byte_serialized", 10 * MUL],

    [.contains_box.base, "contains_box.base", 1200 * MUL],
    [.contains_box.per_byte_serialized, "contains_box.per_byte_serialized", 10 * MUL],

    [.remove_box.base, "remove_box.base", 1200 * MUL],
    [.remove_box.per_byte_serialized, "remove_box.per_byte_serialized", 10 * MUL],

    [.destroy_empty_box.base, "destroy_empty_box.base", 1200 * MUL],

    [.drop_unchecked_box.base, "drop_unchecked_box.base", 100 * MUL],
]);
