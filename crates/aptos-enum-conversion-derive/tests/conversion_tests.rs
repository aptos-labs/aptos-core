// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_enum_conversion_derive::EnumConversion;

#[test]
fn test_enum_conversion_derive_valid() {
    #[derive(Debug, Eq, PartialEq)]
    struct TestMessage {
        x: u32,
    }

    #[derive(Debug, Eq, PartialEq, EnumConversion)]
    enum Messages {
        Test(TestMessage),
    }

    let message = TestMessage { x: 123 };
    let messages = Messages::from(message);
    assert_eq!(messages, Messages::Test(TestMessage { x: 123 }));
}

#[test]
fn test_enum_conversion_derive_invalid() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/cases/*.rs");
}
