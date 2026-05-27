// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    value_serde::{MockFunctionValueExtension, ValueSerDeContext},
    values::{function_values_impl::mock, prop::layout_and_value_strategy},
};
use better_any::TidExt;
use move_core_types::value::MoveValue;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn serializer_round_trip((layout, value) in layout_and_value_strategy()) {
        // Set up mock function extension for function value serialization
        let mut ext_mock = MockFunctionValueExtension::new();
        ext_mock
            .expect_get_serialization_data()
            .returning(move |af| {
                Ok(af
                    .downcast_ref::<mock::MockAbstractFunction>()
                    .expect("Should be a mock abstract function")
                    .data.clone())
            });
        ext_mock
            .expect_create_from_serialization_data()
            .returning(move |data| Ok(Box::new(mock::MockAbstractFunction::new_from_data(data))));

        let ctx = ValueSerDeContext::new(None).with_func_args_deserialization(&ext_mock);
        let blob = ctx.serialize(&value, &layout).unwrap().expect("must serialize");
        let value_deserialized = ValueSerDeContext::new(None).with_func_args_deserialization(&ext_mock).deserialize(&blob, &layout).expect("must deserialize");
        assert!(value.equals(&value_deserialized).unwrap());

        let move_value = value.as_move_value(&layout);

        let blob2 = move_value.simple_serialize().expect("must serialize");
        assert_eq!(blob, blob2);

        let move_value_deserialized = MoveValue::simple_deserialize(&blob2, &layout).expect("must deserialize.");
        assert_eq!(move_value, move_value_deserialized);
    }
}
