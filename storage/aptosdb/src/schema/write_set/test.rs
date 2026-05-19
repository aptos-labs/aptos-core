// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};
use proptest::prelude::*;

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

/// Data serialized with the legacy V0 format must still decode as a top-level `WriteSet::V0`.
#[test]
fn test_decode_legacy_format() {
    let ws = WriteSet::new(vec![
        (
            StateKey::raw(b"key1"),
            WriteOp::legacy_creation(b"val1".to_vec().into()),
        ),
        (
            StateKey::raw(b"key2"),
            WriteOp::legacy_modification(b"val2".to_vec().into()),
        ),
    ])
    .unwrap();

    let old_bytes = bcs::to_bytes(&ws).unwrap();
    let decoded: WriteSet = bcs::from_bytes(&old_bytes).unwrap();

    assert_eq!(decoded.as_v0(), ws.as_v0());
    assert_eq!(decoded.hotness_keys().count(), 0);
}
