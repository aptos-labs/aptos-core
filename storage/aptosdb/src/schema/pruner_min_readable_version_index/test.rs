// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        tag in any::<PrunerTag>(),
        version in any::<PrunedUtilVersion>(),
    ) {
        assert_encode_decode::<PrunerMinReadableVersionIndexSchema>(&tag, &version);
    }
}

test_no_panic_decoding!(PrunerMinReadableVersionIndexSchema);
