// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_enum_conversion_derive::EnumConversion;

#[test]
fn test_enum_conversion_derive_valid() {
    struct _TestMessage {}

    #[derive(EnumConversion)]
    enum _Messages {
        Test(_TestMessage),
    }
}

#[test]
fn test_enum_conversion_derive_invalid() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/cases/*.rs");
}
