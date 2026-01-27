// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use aptos_types::transaction::{PersistedAuxiliaryInfo, Version};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode(version in any::<Version>(), info in any::<PersistedAuxiliaryInfo>()) {
        assert_encode_decode::<PersistedAuxiliaryInfoSchema>(&version, &info);
    }
}

test_no_panic_decoding!(PersistedAuxiliaryInfoSchema);
