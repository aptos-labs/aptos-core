// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use proptest::prelude::*;
use schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};

proptest! {
    #[test]
    fn test_encode_decode(
        version in any::<Version>(),
        epoch_num in any::<u64>(),
    ) {
        assert_encode_decode::<EpochByVersionSchema>(&version, &epoch_num);
    }
}

test_no_panic_decoding!(EpochByVersionSchema);
