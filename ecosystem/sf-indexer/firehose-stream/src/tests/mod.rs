// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod proto_converter_tests;

pub use aptos_api_test_context::{new_test_context as super_new_test_context, TestContext};

fn new_test_context(test_name: String) -> TestContext {
    super_new_test_context(test_name, true)
}
