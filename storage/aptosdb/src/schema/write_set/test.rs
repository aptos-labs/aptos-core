// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        version in any::<Version>(),
        write_set in any::<WriteSet>(),
    ) {
        assert_encode_decode::<WriteSetSchema>(&version, &write_set);
    }
}

test_no_panic_decoding!(WriteSetSchema);
