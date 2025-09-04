// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(
        tag in any::<DbMetadataKey>(),
        data in any::<DbMetadataValue>(),
    ) {
        assert_encode_decode::<DbMetadataSchema>(&tag, &data);
    }
}

test_no_panic_decoding!(DbMetadataSchema);
