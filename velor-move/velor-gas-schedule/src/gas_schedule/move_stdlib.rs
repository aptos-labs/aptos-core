// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the gas parameters for Move Stdlib.

use crate::{
    gas_feature_versions::{RELEASE_V1_18, RELEASE_V1_24},
    gas_schedule::NativeGasParameters,
};
use velor_gas_algebra::{
    InternalGas, InternalGasPerAbstractValueUnit, InternalGasPerArg, InternalGasPerByte,
    InternalGasPerTypeNode,
};

crate::gas_schedule::macros::define_gas_parameters!(
    MoveStdlibGasParameters,
    "move_stdlib",
    NativeGasParameters => .move_stdlib,
    [
        [bcs_to_bytes_per_byte_serialized: InternalGasPerByte, "bcs.to_bytes.per_byte_serialized", 36],
        [bcs_to_bytes_failure: InternalGas, "bcs.to_bytes.failure", 3676],

        [hash_sha2_256_base: InternalGas, "hash.sha2_256.base", 11028],
        [hash_sha2_256_per_byte: InternalGasPerByte, "hash.sha2_256.per_byte", 183],
        [hash_sha3_256_base: InternalGas, "hash.sha3_256.base", 14704],
        [hash_sha3_256_per_byte: InternalGasPerByte, "hash.sha3_256.per_byte", 165],

        // Note(Gas): this initial value is guesswork.
        [signer_borrow_address_base: InternalGas, "signer.borrow_address.base", 735],

        // Note(Gas): these initial values are guesswork.
        [string_check_utf8_base: InternalGas, "string.check_utf8.base", 1102],
        [string_check_utf8_per_byte: InternalGasPerByte, "string.check_utf8.per_byte", 29],
        [string_is_char_boundary_base: InternalGas, "string.is_char_boundary.base", 1102],
        [string_sub_string_base: InternalGas, "string.sub_string.base", 1470],
        [string_sub_string_per_byte: InternalGasPerByte, "string.sub_string.per_byte", 11],
        [string_index_of_base: InternalGas, "string.index_of.base", 1470],
        [string_index_of_per_byte_pattern: InternalGasPerByte, "string.index_of.per_byte_pattern", 73],
        [string_index_of_per_byte_searched: InternalGasPerByte, "string.index_of.per_byte_searched", 36],

        // Note(Gas): these initial values are guesswork.
        [bcs_serialized_size_base: InternalGas, { RELEASE_V1_18.. => "bcs.serialized_size.base" }, 735],
        [bcs_serialized_size_per_byte_serialized: InternalGasPerByte, { RELEASE_V1_18.. => "bcs.serialized_size.per_byte_serialized" }, 36],
        [bcs_serialized_size_failure: InternalGas, { RELEASE_V1_18.. => "bcs.serialized_size.failure" }, 3676],
        [bcs_constant_serialized_size_base: InternalGas, { RELEASE_V1_24.. => "bcs.constant_serialized_size.base" }, 735],
        [bcs_constant_serialized_size_per_type_node: InternalGasPerTypeNode, { RELEASE_V1_24.. => "bcs.constant_serialized_size.per_type_node" }, 40],

        [cmp_compare_base: InternalGas, { RELEASE_V1_24.. => "cmp.compare.base" }, 367],
        [cmp_compare_per_abs_val_unit: InternalGasPerAbstractValueUnit, { RELEASE_V1_24.. => "cmp.compare.per_abs_val_unit"}, 14],

        [vector_move_range_base: InternalGas, { RELEASE_V1_24.. => "vector.move_range.base" }, 4000],
        [vector_move_range_per_index_moved: InternalGasPerArg, { RELEASE_V1_24.. => "vector.move_range.per_index_moved" }, 10],

        [mem_swap_base: InternalGas, { RELEASE_V1_24.. => "mem.swap.base" }, 1500],
    ]
);
