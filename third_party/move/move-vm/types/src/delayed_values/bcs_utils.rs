// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// TODO[agg_v2](cleanup): deduplicate against aptos-types an figure out where
//                        to place such utilities.
pub(crate) fn size_u32_as_uleb128(mut value: usize) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        // 7 (lowest) bits of data get written in a single byte.
        len += 1;
        value >>= 7;
    }
    len
}

pub(crate) fn bcs_size_of_byte_array(length: usize) -> usize {
    size_u32_as_uleb128(length) + length
}
