// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::gas_meter::EXECUTION_GAS_MULTIPLIER as MUL;
use aptos_move_stdlib::natives::GasParameters;

#[cfg(all(test, not(feature = "testing")))]
const UNIT_TEST_ENTRIES: usize = 0;

#[cfg(all(test, feature = "testing"))]
const UNIT_TEST_ENTRIES: usize = 2;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "move_stdlib", [
    [.bcs.to_bytes.per_byte_serialized, "bcs.to_bytes.per_byte_serialized", 10 * MUL],
    [.bcs.to_bytes.failure, "bcs.to_bytes.failure", 1000 * MUL],

    [.hash.sha2_256.base, "hash.sha2_256.base", 3000 * MUL],
    [.hash.sha2_256.per_byte, "hash.sha2_256.per_byte", 50 * MUL],
    [.hash.sha3_256.base, "hash.sha3_256.base", 4000 * MUL],
    [.hash.sha3_256.per_byte, "hash.sha3_256.per_byte", 45 * MUL],

    // Note(Gas): this initial value is guesswork.
    [.signer.borrow_address.base, "signer.borrow_address.base", 200 * MUL],

    // Note(Gas): these initial values are guesswork.
    [.string.check_utf8.base, "string.check_utf8.base", 300 * MUL],
    [.string.check_utf8.per_byte, "string.check_utf8.per_byte", 8 * MUL],
    [.string.is_char_boundary.base, "string.is_char_boundary.base", 300 * MUL],
    [.string.sub_string.base, "string.sub_string.base", 400 * MUL],
    [.string.sub_string.per_byte, "string.sub_string.per_byte", 3 * MUL],
    [.string.index_of.base, "string.index_of.base", 400 * MUL],
    [.string.index_of.per_byte_pattern, "string.index_of.per_byte_pattern", 20 * MUL],
    [.string.index_of.per_byte_searched, "string.index_of.per_byte_searched", 10 * MUL],
], allow_unmapped = 1 /* bcs */ + 2 /* hash */ + 8 /* vector */ + UNIT_TEST_ENTRIES);
