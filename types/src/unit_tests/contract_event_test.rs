// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{contract_event::ContractEvent, event::EventKey};
use bcs::test_helpers::assert_canonical_encode_decode;
use claims::assert_ok;
use move_core_types::language_storage::TypeTag;
use proptest::prelude::*;

proptest! {
    #[test]
    fn event_bcs_roundtrip(event in any::<ContractEvent>()) {
        assert_canonical_encode_decode(event);
    }
}

#[test]
fn test_event_v1_json_serialize() {
    let event_key = EventKey::random();
    let contract_event = assert_ok!(ContractEvent::new_v1(event_key, 0, TypeTag::Address, vec![
        0u8
    ],));
    let contract_json =
        serde_json::to_string(&contract_event).expect("event serialize to json should succeed.");
    let contract_event2: ContractEvent = serde_json::from_str(contract_json.as_str()).unwrap();
    assert_eq!(contract_event, contract_event2)
}

#[test]
fn test_event_v2_json_serialize() {
    let contract_event = assert_ok!(ContractEvent::new_v2(TypeTag::Address, vec![0u8]));
    let contract_json =
        serde_json::to_string(&contract_event).expect("event serialize to json should succeed.");
    let contract_event2: ContractEvent = serde_json::from_str(contract_json.as_str()).unwrap();
    assert_eq!(contract_event, contract_event2)
}
