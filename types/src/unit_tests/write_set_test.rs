// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    state_store::state_key::StateKey,
    write_set::{HotStateOp, WriteOp, WriteSet},
};
use bcs::test_helpers::assert_canonical_encode_decode;
use proptest::prelude::*;
use std::collections::BTreeMap;

proptest! {
    #[test]
    fn write_set_roundtrip_canonical_serialization(write_set in any::<WriteSet>()) {
        assert_canonical_encode_decode(write_set);
    }
}

/// V1 (hotness inline) survives standard serde, while V0 with side-channel hotness drops it
/// — both are intentional and exercised here.
#[test]
fn write_set_v1_carries_hotness_through_serde() {
    let hot_keys: BTreeMap<StateKey, HotStateOp> = [
        (StateKey::raw(b"hot1"), HotStateOp::make_hot()),
        (StateKey::raw(b"hot2"), HotStateOp::make_hot()),
    ]
    .into_iter()
    .collect();

    // V1: hotness inline → preserved by serde.
    let mut ws_v1 = WriteSet::new(vec![(
        StateKey::raw(b"a"),
        WriteOp::legacy_creation(b"v".to_vec().into()),
    )])
    .unwrap();
    ws_v1.add_hotness(hot_keys.clone(), /*persist_in_write_set=*/ true);
    let decoded: WriteSet = bcs::from_bytes(&bcs::to_bytes(&ws_v1).unwrap()).unwrap();
    assert_eq!(decoded.hotness_keys().count(), 2);
    assert_eq!(decoded, ws_v1);

    // V0 + side-channel hotness: dropped by serde (current behavior on testnet/mainnet).
    let mut ws_v0 = WriteSet::new(vec![(
        StateKey::raw(b"a"),
        WriteOp::legacy_creation(b"v".to_vec().into()),
    )])
    .unwrap();
    ws_v0.add_hotness(hot_keys, /*persist_in_write_set=*/ false);
    let decoded: WriteSet = bcs::from_bytes(&bcs::to_bytes(&ws_v0).unwrap()).unwrap();
    assert_eq!(decoded.hotness_keys().count(), 0);
}
