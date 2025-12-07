// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
