// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::TestContext;

mod accounts_test;
mod events_test;
mod index_test;
mod invalid_post_request_test;
mod state_test;
mod string_resource_test;
mod transaction_vector_test;
mod transactions_test;

pub const API_VERSION: &str = "v0";

pub fn new_test_context(test_name: String) -> TestContext {
    super::new_test_context(test_name, API_VERSION)
}
