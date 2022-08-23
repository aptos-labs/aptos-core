// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_stdlib::natives::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "move_stdlib", [
    [.bcs.to_bytes.per_byte_serialized, "bcs.to_bytes.per_byte_serialized", 1],
    [.bcs.to_bytes.failure, "bcs.to_bytes.failure", 10],

    [.hash.sha2_256.base, "hash.sha2_256.base", 1],
    [.hash.sha2_256.per_byte, "hash.sha2_256.per_byte", 1],
    [.hash.sha3_256.base, "hash.sha3_256.base", 1],
    [.hash.sha3_256.per_byte, "hash.sha3_256.per_byte", 1],

    [.signer.borrow_address.base, "signer.borrow_address.base", 1],

    [.string.check_utf8.base, "string.check_utf8.base", 1],
    [.string.check_utf8.per_byte, "string.check_utf8.per_byte", 1],
    [.string.is_char_boundary.base, "string.is_char_boundary.base", 1],
    [.string.sub_string.base, "string.sub_string.base", 1],
    [.string.sub_string.per_byte, "string.sub_string.per_byte", 1],
    [.string.index_of.base, "string.index_of.base", 1],
    [.string.index_of.per_byte_pattern, "string.index_of.per_byte_pattern", 1],
    [.string.index_of.per_byte_searched, "string.index_of.per_byte_searched", 1],

    // TODO(Gas): these should only be enabled when feature "testing" is present
    // TODO(Gas): rename these in the move repo
    [.unit_test.create_signers_for_testing.base_cost, "unit_test.create_signers_for_testing.base", 1],
    [.unit_test.create_signers_for_testing.unit_cost, "unit_test.create_signers_for_testing.unit", 1]
], allow_unmapped = 1 /* bcs */ + 2 /* hash */ + 8 /* vector */);
