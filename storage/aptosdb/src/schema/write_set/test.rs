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

    let bytes = <WriteSet as ValueCodec<WriteSetSchema>>::encode_value(&ws).unwrap();
    let decoded = <WriteSet as ValueCodec<WriteSetSchema>>::decode_value(&bytes).unwrap();
    assert_eq!(decoded.as_v0(), ws.as_v0());
    assert_eq!(decoded.hotness_keys().count(), 0);
}

/// V1 WriteSets (crafted from a mirror since the production code has no public
/// `WriteSet::V1` constructor yet) round-trip through the schema codec.
#[test]
fn test_v1_roundtrip() {
    use aptos_types::write_set::{Extension, WriteSetMut};

    // Mirrors the on-wire shape of `WriteSetV1`.
    #[derive(serde::Serialize)]
    struct WriteSetV1Mirror<'a> {
        ws_mut: &'a WriteSetMut,
        hotness: &'a BTreeSet<StateKey>,
        extensions: &'a [Extension],
    }

    let ws_mut = WriteSetMut::new(vec![(
        StateKey::raw(b"key1"),
        WriteOp::legacy_creation(b"val1".to_vec().into()),
    )]);
    let hotness: BTreeSet<StateKey> = [StateKey::raw(b"hot1")].into_iter().collect();

    let mut bytes = vec![1u8]; // BCS variant tag for V1.
    bytes.extend(
        bcs::to_bytes(&WriteSetV1Mirror {
            ws_mut: &ws_mut,
            hotness: &hotness,
            extensions: &[],
        })
        .unwrap(),
    );

    let decoded = <WriteSet as ValueCodec<WriteSetSchema>>::decode_value(&bytes).unwrap();
    let reencoded = <WriteSet as ValueCodec<WriteSetSchema>>::encode_value(&decoded).unwrap();
    assert_eq!(reencoded, bytes);
    let redecoded = <WriteSet as ValueCodec<WriteSetSchema>>::decode_value(&reencoded).unwrap();
    assert_eq!(decoded, redecoded);
}

/// Legacy V1 bytes (variant tag 1 with `value ++ hotness`, no trailing extensions) must
/// decode via the fallback path and preserve both writes and hotness.
#[test]
fn test_decode_legacy_v1_fallback() {
    // Mirrors the legacy V1 payload shape so we can serialize it from this side.
    #[derive(serde::Serialize)]
    struct LegacyV1<'a> {
        value: &'a WriteSetV0,
        hotness: &'a BTreeSet<StateKey>,
    }

    let writes = WriteSet::new(vec![(
        StateKey::raw(b"key1"),
        WriteOp::legacy_creation(b"val1".to_vec().into()),
    )])
    .unwrap();
    let hotness: BTreeSet<StateKey> = [StateKey::raw(b"hot1"), StateKey::raw(b"hot2")]
        .into_iter()
        .collect();

    let mut bytes = vec![1u8]; // BCS variant tag for V1.
    bytes.extend(
        bcs::to_bytes(&LegacyV1 {
            value: writes.as_v0(),
            hotness: &hotness,
        })
        .unwrap(),
    );

    let mut expected = writes;
    expected.add_hotness(hotness);
    let decoded = <WriteSet as ValueCodec<WriteSetSchema>>::decode_value(&bytes).unwrap();
    assert_eq!(decoded.as_v0(), expected.as_v0());
}
