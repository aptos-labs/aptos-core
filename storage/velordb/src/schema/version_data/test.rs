// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use velor_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use velor_types::transaction::Version;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(version in any::<Version>(), version_data in any::<VersionData>()) {
        assert_encode_decode::<VersionDataSchema>(&version, &version_data);
    }
}

test_no_panic_decoding!(VersionDataSchema);
