// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use aptos_schemadb::{schema::fuzzing::assert_encode_decode, test_no_panic_decoding};
use aptos_types::write_set::WriteOp;
use proptest::prelude::*;
use std::collections::BTreeMap;

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

/// Data serialized with the old format (`bcs::to_bytes` via the custom `WriteSet::Serialize`,
/// which only serializes the `value` field) must decode correctly through `decode_write_set`.
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

    // Old encode path: WriteSet::Serialize → ValueWriteSet::V0
    let old_bytes = bcs::to_bytes(&ws).unwrap();
    let decoded = decode_write_set(&old_bytes).unwrap();

    assert_eq!(decoded.as_v0(), ws.as_v0());
    assert_eq!(decoded.hotness_keys().count(), 0);
}

/// Roundtrip: a `WriteSet` with hotness, encoded via `encode_write_set(..., true)` and decoded
/// via `decode_write_set`, must preserve both the value write ops and the set of hot keys.
#[test]
fn test_roundtrip_with_hotness() {
    let mut ws = WriteSet::new(vec![(
        StateKey::raw(b"a"),
        WriteOp::legacy_creation(b"v".to_vec().into()),
    )])
    .unwrap();

    let hot_keys: BTreeMap<StateKey, HotStateOp> = [
        (StateKey::raw(b"hot1"), HotStateOp::make_hot()),
        (StateKey::raw(b"hot2"), HotStateOp::make_hot()),
    ]
    .into_iter()
    .collect();
    ws.add_hotness(hot_keys);

    let encoded = encode_write_set(&ws, true).unwrap();
    let decoded = decode_write_set(&encoded).unwrap();

    assert_eq!(decoded.as_v0(), ws.as_v0());
    assert_eq!(
        decoded.hotness_keys().collect::<BTreeSet<_>>(),
        ws.hotness_keys().collect::<BTreeSet<_>>(),
    );
}

/// `encode_write_set(ws, false)` must produce byte-identical output to the old
/// `bcs::to_bytes(&ws)` path. This guarantees `PersistedWriteSet::V0` has the same BCS layout
/// as `ValueWriteSet::V0`.
#[test]
fn test_v0_byte_identity() {
    let ws = WriteSet::new(vec![
        (
            StateKey::raw(b"x"),
            WriteOp::legacy_creation(b"y".to_vec().into()),
        ),
        (StateKey::raw(b"z"), WriteOp::legacy_deletion()),
    ])
    .unwrap();

    let old_bytes = bcs::to_bytes(&ws).unwrap();
    let new_bytes = encode_write_set(&ws, false).unwrap();
    assert_eq!(old_bytes, new_bytes);
}
