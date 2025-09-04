// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_enum_conversion_derive::EnumConversion;

#[test]
fn test_enum_conversion_derive_valid() {
    struct TestMessage {}

    #[derive(EnumConversion)]
    enum Messages {
        Test(TestMessage),
    }
}

#[test]
fn test_enum_conversion_derive_invalid() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/cases/*.rs");
}
