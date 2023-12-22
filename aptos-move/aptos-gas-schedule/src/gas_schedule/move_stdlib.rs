// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the gas parameters for Move Stdlib.

use crate::gas_schedule::NativeGasParameters;
use aptos_gas_algebra::{InternalGas, InternalGasPerByte};

crate::gas_schedule::macros::define_gas_parameters!(
    MoveStdlibGasParameters,
    "move_stdlib",
    NativeGasParameters => .move_stdlib,
    [
        [bcs_to_bytes_per_byte_serialized: InternalGasPerByte, "bcs.to_bytes.per_byte_serialized", 41],
        [bcs_to_bytes_failure: InternalGas, "bcs.to_bytes.failure", 4180],

        [hash_sha2_256_base: InternalGas, "hash.sha2_256.base", 12540],
        [hash_sha2_256_per_byte: InternalGasPerByte, "hash.sha2_256.per_byte", 209],
        [hash_sha3_256_base: InternalGas, "hash.sha3_256.base", 16720],
        [hash_sha3_256_per_byte: InternalGasPerByte, "hash.sha3_256.per_byte", 188],

        // Note(Gas): this initial value is guesswork.
        [signer_borrow_address_base: InternalGas, "signer.borrow_address.base", 836],

        // Note(Gas): these initial values are guesswork.
        [string_check_utf8_base: InternalGas, "string.check_utf8.base", 1254],
        [string_check_utf8_per_byte: InternalGasPerByte, "string.check_utf8.per_byte", 33],
        [string_is_char_boundary_base: InternalGas, "string.is_char_boundary.base", 1254],
        [string_sub_string_base: InternalGas, "string.sub_string.base", 1672],
        [string_sub_string_per_byte: InternalGasPerByte, "string.sub_string.per_byte", 12],
        [string_index_of_base: InternalGas, "string.index_of.base", 1672],
        [string_index_of_per_byte_pattern: InternalGasPerByte, "string.index_of.per_byte_pattern", 83],
        [string_index_of_per_byte_searched: InternalGasPerByte, "string.index_of.per_byte_searched", 41],
    ]
);
