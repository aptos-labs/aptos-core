// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub use move_binary_format::file_format_common::{bcs_size_of_byte_array, size_u32_as_uleb128};

// TODO[agg_v2](cleanup): check if tests need to go to move_binary_format and
//                        if we need pub use here.

#[test]
fn test_size_u32_as_uleb128() {
    assert_eq!(size_u32_as_uleb128(0), 1);
    assert_eq!(size_u32_as_uleb128(127), 1);
    assert_eq!(size_u32_as_uleb128(128), 2);
    assert_eq!(size_u32_as_uleb128(128 * 128 - 1), 2);
    assert_eq!(size_u32_as_uleb128(128 * 128), 3);
}

#[test]
fn test_group_size_same_as_bcs() {
    use bytes::Bytes;

    let reused_vec = Bytes::from(vec![5; 20000]);

    for i in [1, 2, 3, 5, 15, 100, 1000, 10000, 20000] {
        assert_eq!(
            bcs::serialized_size(&reused_vec.slice(0..i)).unwrap(),
            bcs_size_of_byte_array(i)
        );
    }
}
