// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{value_serde::ValueSerDeContext, values::prop::layout_and_value_strategy};
use move_core_types::value::MoveValue;
use proptest::prelude::*;

proptest! {
    #[test]
    fn serializer_round_trip((layout, value) in layout_and_value_strategy()) {
        let blob = ValueSerDeContext::new().serialize(&value, &layout).unwrap().expect("must serialize");
        let value_deserialized = ValueSerDeContext::new().deserialize(&blob, &layout).expect("must deserialize");
        assert!(value.equals(&value_deserialized).unwrap());

        let move_value = value.as_move_value(&layout);

        let blob2 = move_value.simple_serialize().expect("must serialize");
        assert_eq!(blob, blob2);

        let move_value_deserialized = MoveValue::simple_deserialize(&blob2, &layout).expect("must deserialize.");
        assert_eq!(move_value, move_value_deserialized);
    }
}
