// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_stdlib::natives::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "move_stdlib", [
    [.bcs.to_bytes.input_unit_cost, "bcs.to_bytes.input_unit", 1],
    [.bcs.to_bytes.output_unit_cost, "bcs.to_bytes.output_unit", 0],
    [.bcs.to_bytes.failure_cost, "bcs.to_bytes.failure", 10],

    [.hash.sha2_256.base_cost, "hash.sha2_256.base", 1],
    [.hash.sha2_256.unit_cost, "hash.sha2_256.unit", 1],
    [.hash.sha3_256.base_cost, "hash.sha3_256.base", 1],
    [.hash.sha3_256.unit_cost, "hash.sha3_256.unit", 1],

    [.signer.borrow_address.base_cost, "signer.borrow_address.base", 1],

    [.string.check_utf8.base_cost, "string.check_utf8.base", 1],
    [.string.check_utf8.unit_cost, "string.check_utf8.unit", 1],
    [.string.is_char_boundary.base_cost, "string.is_char_boundary.base", 1],
    [.string.sub_string.base_cost, "string.sub_string.base", 1],
    [.string.sub_string.unit_cost, "string.sub_string.unit", 1],
    [.string.index_of.base_cost, "string.index_of.base", 1],
    [.string.index_of.unit_cost, "string.index_of.unit", 1],

    // TODO(Gas): these should only be enabled when feature "testing" is present
    [.unit_test.create_signers_for_testing.base_cost, "unit_test.create_signers_for_testing.base", 1],
    [.unit_test.create_signers_for_testing.unit_cost, "unit_test.create_signers_for_testing.unit", 1]
], allow_unmapped = 1 /* bcs */ + 2 /* hash */ + 8 /* vector */);
