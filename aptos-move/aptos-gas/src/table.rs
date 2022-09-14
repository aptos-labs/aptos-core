// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_table_extension::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "table", [
    [.common.load_base, "common.load.base", 1],
    [.common.load_per_byte, "common.load.per_byte", 1],
    [.common.load_failure, "common.load.failure", 1],

    [.new_table_handle.base, "new_table_handle.base", 1],

    [.add_box.base, "add_box.base", 1],
    [.add_box.per_byte_serialized, "add_box.per_byte_serialized", 1],

    [.borrow_box.base, "borrow_box.base", 1],
    [.borrow_box.per_byte_serialized, "borrow_box.per_byte_serialized", 1],

    [.contains_box.base, "contains_box.base", 1],
    [.contains_box.per_byte_serialized, "contains_box.per_byte_serialized", 1],

    [.remove_box.base, "remove_box.base", 1],
    [.remove_box.per_byte_serialized, "remove_box.per_byte_serialized", 1],

    [.destroy_empty_box.base, "destroy_empty_box.base", 1],

    [.drop_unchecked_box.base, "drop_unchecked_box.base", 1],
]);
