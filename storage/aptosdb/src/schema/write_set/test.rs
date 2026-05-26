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

/// V0 (default) WriteSets round-trip through the schema codec.
#[test]
fn test_v0_roundtrip() {
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

    let bytes = ws.encode_value().unwrap();
    let decoded = WriteSet::decode_value(&bytes).unwrap();
    assert_eq!(decoded.as_v0(), ws.as_v0());
    assert_eq!(decoded.hotness_keys().count(), 0);
}

/// V1 WriteSets constructed by production code round-trip through the schema codec.
#[test]
fn test_v1_roundtrip() {
    let mut ws = WriteSet::new(vec![(
        StateKey::raw(b"key1"),
        WriteOp::legacy_creation(b"val1".to_vec().into()),
    )])
    .unwrap();
    let hotness: BTreeSet<_> = [StateKey::raw(b"hot1")].into_iter().collect();
    ws.add_hotness(hotness.clone());
    let ws = ws.into_v1();

    let bytes = ws.encode_value().unwrap();
    let decoded = WriteSet::decode_value(&bytes).unwrap();
    assert!(matches!(decoded, WriteSet::V1(_)));
    assert_eq!(decoded, ws);
    assert_eq!(
        decoded.hotness_keys().cloned().collect::<BTreeSet<_>>(),
        hotness
    );
}

/// Legacy V1 bytes (variant tag 1 with `value ++ hotness`, no trailing extensions) must
/// decode via the fallback path and preserve both writes and hotness.
#[test]
fn test_decode_legacy_v1_fallback() {
    let writes = WriteSet::new(vec![(
        StateKey::raw(b"key1"),
        WriteOp::legacy_creation(b"val1".to_vec().into()),
    )])
    .unwrap();
    let hotness: BTreeSet<_> = [StateKey::raw(b"hot1"), StateKey::raw(b"hot2")]
        .into_iter()
        .collect();

    let mut bytes = vec![1u8]; // BCS variant tag for V1.
    bytes.extend(
        bcs::to_bytes(&LegacyWriteSetV1Payload {
            value: writes.as_v0().clone(),
            hotness: hotness.clone(),
        })
        .unwrap(),
    );

    let mut expected = writes;
    expected.add_hotness(hotness);
    let decoded = WriteSet::decode_value(&bytes).unwrap();
    assert_eq!(decoded.as_v0(), expected.as_v0());
}
